use unicode_width::UnicodeWidthStr;

use crate::config::{Config, Severity};

#[derive(Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

struct Header<'a> {
    kind: &'a str,
    scope: Option<&'a str>,
    subject: &'a str,
}

fn parse_header(line: &str) -> Option<Header<'_>> {
    let bytes = line.as_bytes();
    if !bytes.first()?.is_ascii_alphabetic() {
        return None;
    }
    let mut pos = 1;
    while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'-') {
        pos += 1;
    }
    let kind = &line[..pos];
    let mut scope = None;
    if bytes.get(pos) == Some(&b'(') {
        let start = pos + 1;
        let end = line[start..].find(')')? + start;
        let value = &line[start..end];
        if value.is_empty() || value.chars().any(|c| c.is_whitespace() || c == '(') {
            return None;
        }
        scope = Some(value);
        pos = end + 1;
    }
    if bytes.get(pos) == Some(&b'!') {
        pos += 1;
    }
    if !line[pos..].starts_with(": ") {
        return None;
    }
    let subject = &line[pos + 2..];
    if subject.starts_with(' ') {
        return None;
    }
    Some(Header {
        kind,
        scope,
        subject,
    })
}

fn trailer_token_rest(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("BREAKING CHANGE") {
        return Some(rest);
    }
    let end = line
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || matches!(c, ':' | '#'))
        .map_or(line.len(), |(i, _)| i);
    (end > 0).then_some(&line[end..])
}

fn trailer_candidate(line: &str) -> bool {
    let Some(rest) = trailer_token_rest(line) else {
        return false;
    };
    rest.starts_with(':')
        || rest.starts_with('#')
        || (rest.starts_with(' ')
            && (rest.trim_start_matches(' ').starts_with(':')
                || rest.trim_start_matches(' ').starts_with('#')))
}

fn valid_trailer(line: &str) -> bool {
    let Some(rest) = trailer_token_rest(line) else {
        return false;
    };
    rest.strip_prefix(": ")
        .or_else(|| rest.strip_prefix(" #"))
        .is_some_and(|value| !value.trim().is_empty())
}

fn forbidden_control(c: char) -> bool {
    (c <= '\u{1f}' || ('\u{7f}'..='\u{9f}').contains(&c))
        || ('\u{202a}'..='\u{202e}').contains(&c)
        || ('\u{2066}'..='\u{2069}').contains(&c)
}

struct Linter<'a> {
    config: &'a Config,
    diagnostics: Vec<Diagnostic>,
}

impl Linter<'_> {
    fn report(&mut self, line: usize, rule: &'static str, message: impl Into<String>) {
        let settings = &self.config.rules[rule];
        if settings.enabled {
            self.diagnostics.push(Diagnostic {
                line,
                rule,
                severity: settings.severity,
                message: message.into(),
            });
        }
    }

    fn width(&mut self, line: usize, rule: &'static str, text: &str) {
        let settings = &self.config.rules[rule];
        if !settings.enabled {
            return;
        }
        let width = if self.config.ambiguous_width == 2 {
            text.width_cjk()
        } else {
            text.width()
        };
        if let Some(threshold) = settings.thresholds.iter().rev().find(|t| width > t.limit) {
            self.diagnostics.push(Diagnostic {
                line,
                rule,
                severity: threshold.severity,
                message: format!("display width {width} exceeds {}", threshold.limit),
            });
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Line<'a> {
    number: usize,
    // The raw text excludes LF but retains the CR of a CRLF ending.
    raw: &'a str,
    has_lf: bool,
}

impl<'a> Line<'a> {
    fn text(self) -> &'a str {
        if self.has_lf {
            self.raw.strip_suffix('\r').unwrap_or(self.raw)
        } else {
            self.raw
        }
    }
}

fn input_lines(input: &str) -> Vec<Line<'_>> {
    if input.is_empty() {
        return vec![Line {
            number: 1,
            raw: "",
            has_lf: false,
        }];
    }
    input
        .split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| Line {
            number: index + 1,
            raw: line.strip_suffix('\n').unwrap_or(line),
            has_lf: line.ends_with('\n'),
        })
        .collect()
}

fn filtered_lines<'a>(
    input: &'a str,
    comment_prefix: &str,
    strip_comments: bool,
    cut_scissors: bool,
) -> Vec<Line<'a>> {
    const SCISSORS: &str = " ------------------------ >8 ------------------------";
    let mut lines = input_lines(input);
    if cut_scissors
        && let Some(index) = lines
            .iter()
            .position(|line| line.has_lf && line.raw.strip_prefix(comment_prefix) == Some(SCISSORS))
    {
        lines.truncate(index);
    }
    if strip_comments {
        lines.retain(|line| !line.raw.starts_with(comment_prefix));
    }
    lines
}

/// Lints the original input without removing comments or scissors lines.
pub fn lint(input: &str, config: &Config) -> Vec<Diagnostic> {
    lint_lines(&input_lines(input), config)
}

/// Lints input after optionally removing comment lines and cutting at scissors.
///
/// Matching uses the supplied literal prefix and the original line endings.
/// Diagnostics retain their one-based line numbers in the original input.
pub fn lint_filtered(
    input: &str,
    config: &Config,
    comment_prefix: &str,
    strip_comments: bool,
    cut_scissors: bool,
) -> Vec<Diagnostic> {
    lint_lines(
        &filtered_lines(input, comment_prefix, strip_comments, cut_scissors),
        config,
    )
}

fn lint_lines(lines: &[Line<'_>], config: &Config) -> Vec<Diagnostic> {
    let mut linter = Linter {
        config,
        diagnostics: Vec::new(),
    };
    let all_filtered = lines.is_empty();
    let header = lines.first().copied().unwrap_or(Line {
        number: 1,
        raw: "",
        has_lf: false,
    });
    let header_line = header.number;
    let header = header.text();

    if header.trim() != header {
        linter.report(
            header_line,
            "header-trim",
            "header has leading or trailing whitespace",
        );
    }
    linter.width(header_line, "header-max-width", header);
    if let Some(parsed) = parse_header(header) {
        if !config
            .types
            .iter()
            .any(|kind| kind.eq_ignore_ascii_case(parsed.kind))
        {
            linter.report(header_line, "type-enum", "type is not in the allowed list");
        }
        if parsed.kind.bytes().any(|b| b.is_ascii_uppercase()) {
            linter.report(header_line, "type-case", "type must be lowercase");
        }
        if parsed.scope.is_none() {
            linter.report(header_line, "scope-required", "scope is required");
        }
        if let Some(scope) = parsed.scope
            && let Some(allowed) = &config.allowed_scopes
            && !allowed.iter().any(|value| value == scope)
        {
            linter.report(
                header_line,
                "scope-enum",
                "scope is not in the allowed list",
            );
        }
        if parsed.subject.is_empty() {
            linter.report(header_line, "subject-empty", "subject must not be empty");
        }
        if parsed.subject.ends_with('.') {
            linter.report(
                header_line,
                "subject-full-stop",
                "subject must not end with a full stop",
            );
        }
    } else {
        linter.report(
            header_line,
            "header-format",
            if all_filtered {
                "header does not match the Conventional Commit format (all lines were removed by filtering)"
            } else {
                "header does not match the Conventional Commit format"
            },
        );
    }

    for (index, line) in lines.iter().enumerate() {
        let text = line.text();
        if text.chars().any(forbidden_control) {
            linter.report(
                line.number,
                "control-character",
                "line contains a forbidden control character",
            );
        }
        if index > 0 && text.trim_end() != text {
            linter.report(
                line.number,
                "line-trailing-whitespace",
                "line has trailing whitespace",
            );
        }
    }

    let first_content = (1..lines.len()).find(|&i| !lines[i].text().is_empty());
    if let Some(first) = first_content {
        if first != 2 {
            linter.report(
                lines[first].number,
                "section-separator",
                "header and following section must be separated by one empty line",
            );
        }
        let footer = (first..lines.len()).find(|&i| {
            valid_trailer(lines[i].text())
                || (trailer_candidate(lines[i].text())
                    && (i == first || lines[i - 1].text().is_empty()))
        });
        if let Some(start) = footer
            && start > first
        {
            let mut separator_start = start;
            while separator_start > first && lines[separator_start - 1].text().is_empty() {
                separator_start -= 1;
            }
            if start - separator_start != 1 {
                linter.report(
                    lines[start].number,
                    "section-separator",
                    "body and footer must be separated by one empty line",
                );
            }
        }
        let mut trailer_started = false;
        for (index, line) in lines.iter().enumerate().skip(first) {
            let text = line.text();
            if footer.is_some_and(|start| index >= start) {
                linter.width(line.number, "footer-max-width", text);
                if valid_trailer(text) {
                    trailer_started = true;
                } else if !trailer_started && index == footer.unwrap() {
                    linter.report(line.number, "trailer-format", "invalid trailer");
                }
            } else {
                linter.width(line.number, "body-max-width", text);
            }
        }
    }
    linter
        .diagnostics
        .sort_by(|a, b| a.line.cmp(&b.line).then(a.rule.cmp(b.rule)));
    linter.diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(input: &str) -> Vec<(usize, &'static str, Severity)> {
        lint(input, &Config::default())
            .into_iter()
            .map(|d| (d.line, d.rule, d.severity))
            .collect()
    }

    fn filtered_rules(input: &str) -> Vec<(usize, &'static str, Severity)> {
        lint_filtered(input, &Config::default(), "#", true, true)
            .into_iter()
            .map(|d| (d.line, d.rule, d.severity))
            .collect()
    }

    fn remaining(input: &str, prefix: &str, strip: bool, cut: bool) -> Vec<(usize, String)> {
        filtered_lines(input, prefix, strip, cut)
            .into_iter()
            .map(|line| (line.number, line.text().to_owned()))
            .collect()
    }

    #[test]
    fn filtering_modes_are_independent_and_cut_at_the_first_scissors() {
        let marker = "# ------------------------ >8 ------------------------";
        let input =
            format!("# intro\nfeat: change\n# note\n\nbody\n{marker}\ndiff\n{marker}\nend\n");
        let expected = [
            (false, false, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]),
            (true, false, vec![2, 4, 5, 7, 9]),
            (false, true, vec![1, 2, 3, 4, 5]),
            (true, true, vec![2, 4, 5]),
        ];
        for (strip, cut, numbers) in expected {
            let actual: Vec<_> = remaining(&input, "#", strip, cut)
                .into_iter()
                .map(|(number, _)| number)
                .collect();
            assert_eq!(actual, numbers, "strip={strip}, cut={cut}");
        }
    }

    #[test]
    fn lint_applies_each_filter_independently_before_classifying_sections() {
        let input = format!(
            "# intro\nfeat: change\n\nbody\n# ------------------------ >8 ------------------------\n{}\n",
            "x".repeat(73)
        );
        let cases = [
            (
                false,
                false,
                vec![
                    (1, "header-format", Severity::Error),
                    (2, "section-separator", Severity::Error),
                    (6, "footer-max-width", Severity::Error),
                ],
            ),
            (true, false, vec![(6, "body-max-width", Severity::Error)]),
            (
                false,
                true,
                vec![
                    (1, "header-format", Severity::Error),
                    (2, "section-separator", Severity::Error),
                ],
            ),
            (true, true, vec![]),
        ];
        for (strip, cut, expected) in cases {
            let actual: Vec<_> = lint_filtered(&input, &Config::default(), "#", strip, cut)
                .into_iter()
                .map(|d| (d.line, d.rule, d.severity))
                .collect();
            assert_eq!(actual, expected, "strip={strip}, cut={cut}");
        }
    }

    #[test]
    fn comments_match_the_literal_prefix_only_at_the_start_of_raw_lines() {
        let input = "##note\n## note\n#note\n ##note\n\n\r\n##";
        assert_eq!(
            remaining(input, "##", true, false),
            vec![
                (3, "#note".into()),
                (4, " ##note".into()),
                (5, "".into()),
                (6, "".into()),
            ]
        );
        assert_eq!(
            remaining("#note\n #note", "#", true, false),
            vec![(2, " #note".into())]
        );
        assert_eq!(
            remaining("\r\nkeep", "\r", true, false),
            vec![(2, "keep".into())]
        );
    }

    #[test]
    fn scissors_requires_exact_raw_line_with_lf_and_the_literal_prefix() {
        let marker = "## ------------------------ >8 ------------------------";
        for suffix in ["\n", "\nafter\n"] {
            let input = format!("before\n{marker}{suffix}");
            assert_eq!(
                remaining(&input, "##", false, true),
                vec![(1, "before".into())]
            );
        }
        let nonmarkers = [
            format!("{marker}\r\n"),
            marker.to_string(),
            format!(" {marker}\n"),
            format!("{marker} extra\n"),
            format!("{marker} \n"),
            "# ------------------------ >8 ------------------------\n".to_string(),
            "##  ------------------------ >8 ------------------------\n".to_string(),
            "## ------------------------ >8 -----------------------\n".to_string(),
        ];
        for line in nonmarkers {
            let input = format!("before\n{line}");
            assert_eq!(remaining(&input, "##", false, true).len(), 2, "{line:?}");
        }
        assert_eq!(
            remaining(&format!("{marker}\r\nafter\n"), "##", true, true),
            vec![(2, "after".into())],
            "a CRLF marker remains eligible for comment stripping"
        );
        assert!(
            remaining(
                "☂ ------------------------ >8 ------------------------\n",
                "☂",
                false,
                true
            )
            .is_empty()
        );
    }

    #[test]
    fn line_splitting_matches_original_normalization_for_all_short_inputs() {
        // Compare with the original API's replace, single trailing-LF removal and split.
        for len in 0..=6 {
            for mut sequence in 0..4_usize.pow(len) {
                let mut input = String::new();
                for _ in 0..len {
                    input.push(['x', '\r', '\n', 'あ'][sequence % 4]);
                    sequence /= 4;
                }
                let normalized = input.replace("\r\n", "\n");
                let content = normalized.strip_suffix('\n').unwrap_or(&normalized);
                let expected: Vec<_> = content.split('\n').collect();
                let lines = input_lines(&input);
                let actual: Vec<_> = lines.iter().map(|line| line.text()).collect();
                assert_eq!(actual, expected, "input={input:?}");
            }
        }
    }

    #[test]
    fn disabling_both_filters_matches_raw_lint_including_messages_and_severities() {
        let config = Config::default();
        for input in [
            "",
            "\n",
            "# note\nfeat: change\n",
            "feat: change\r\n\r\nbody \r\n",
            "feat: change\n# ------------------------ >8 ------------------------\ndiff\r",
            "feat: change\n\n\n",
        ] {
            let describe = |diagnostics: Vec<Diagnostic>| {
                diagnostics
                    .into_iter()
                    .map(|d| (d.line, d.rule, d.severity, d.message))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                describe(lint(input, &config)),
                describe(lint_filtered(input, &config, "#", false, false)),
                "input={input:?}"
            );
        }
        assert_eq!(rules("# note\nfeat: change")[0].1, "header-format");
    }

    #[test]
    fn all_removed_lines_get_a_distinct_header_error_unless_the_rule_is_disabled() {
        let config = Config::default();
        let ordinary_empty = lint_filtered("", &config, "#", true, true);
        assert_eq!(ordinary_empty.len(), 1);
        assert_eq!(ordinary_empty[0].line, 1);
        assert_eq!(ordinary_empty[0].rule, "header-format");
        assert!(!ordinary_empty[0].message.contains("filtering"));
        for input in [
            "# comment",
            "# comment\n# another\n",
            "# ------------------------ >8 ------------------------\nafter\n",
        ] {
            let diagnostics = lint_filtered(input, &config, "#", true, true);
            assert_eq!(diagnostics.len(), 1, "{input:?}");
            assert_eq!(
                (diagnostics[0].line, diagnostics[0].rule),
                (1, "header-format")
            );
            assert!(
                diagnostics[0]
                    .message
                    .contains("all lines were removed by filtering")
            );
        }
        let cut_only = lint_filtered(
            "# ------------------------ >8 ------------------------\n",
            &config,
            "#",
            false,
            true,
        );
        assert_eq!((cut_only[0].line, cut_only[0].rule), (1, "header-format"));
        assert!(
            cut_only[0]
                .message
                .contains("all lines were removed by filtering")
        );
        let warning =
            Config::parse("version = 1\n[rules.header-format]\nseverity = 'warning'").unwrap();
        assert_eq!(
            lint_filtered("# comment\n", &warning, "#", true, true)[0].severity,
            Severity::Warning
        );
        let disabled =
            Config::parse("version = 1\n[rules.header-format]\nenabled = false").unwrap();
        assert!(lint_filtered("# comment\n", &disabled, "#", true, true).is_empty());
        let remaining_empty = lint_filtered("# comment\n\n", &config, "#", true, true);
        assert_eq!(remaining_empty[0].line, 2);
        assert!(!remaining_empty[0].message.contains("filtering"));
    }

    #[test]
    fn filtered_structure_uses_retained_lines_without_skipping_or_inventing_blanks() {
        assert!(filtered_rules("# intro\nfeat: change\n# separator\n\n# body\nbody\n# footer separator\n\n# footer\nRefs: value\n# end\n").is_empty());
        assert_eq!(
            filtered_rules("# intro\n\nfeat: change\n"),
            vec![
                (2, "header-format", Severity::Error),
                (3, "section-separator", Severity::Error)
            ]
        );
        assert_eq!(
            filtered_rules("feat: change\n# note\nbody\n"),
            vec![(3, "section-separator", Severity::Error)]
        );
        assert_eq!(
            filtered_rules("feat: change\n\n# note\n\nbody\n"),
            vec![(5, "section-separator", Severity::Error)]
        );
        assert_eq!(
            filtered_rules("feat: change\n\nbody\n# note\nRefs: value\n"),
            vec![(5, "section-separator", Severity::Error)]
        );
        assert_eq!(
            filtered_rules("feat: change\n\n# note\nRefs:#123\n"),
            vec![(4, "trailer-format", Severity::Error)]
        );
        assert!(filtered_rules("feat: change\n# only remaining line is the header\n").is_empty());
    }

    #[test]
    fn all_header_rules_report_the_original_header_line() {
        let config = Config::parse(
            "version = 1\n[conventional]\nscope-required = true\nallowed-scopes = ['cli']",
        )
        .unwrap();
        let cases = [
            ("bad", "header-format"),
            (" feat: change", "header-trim"),
            ("other: change", "type-enum"),
            ("Feat: change", "type-case"),
            ("feat: change", "scope-required"),
            ("feat(other): change", "scope-enum"),
            ("feat: ", "subject-empty"),
            ("feat: change.", "subject-full-stop"),
            ("feat: \u{202e}change", "control-character"),
        ];
        for (header, rule) in cases {
            let diagnostics = lint_filtered(
                &format!("# first\n# second\n{header}\n"),
                &config,
                "#",
                true,
                true,
            );
            assert!(
                diagnostics.iter().any(|d| d.line == 3 && d.rule == rule),
                "header={header:?}, rule={rule}: {diagnostics:?}"
            );
        }
        for (length, severity) in [(45, Severity::Warning), (67, Severity::Error)] {
            let input = format!("# ignored\nfeat: {}\n", "x".repeat(length));
            let diagnostics = lint_filtered(&input, &config, "#", true, true);
            assert!(
                diagnostics
                    .iter()
                    .any(|d| (d.line, d.rule, d.severity) == (2, "header-max-width", severity))
            );
        }
    }

    #[test]
    fn body_footer_and_separator_diagnostics_retain_original_locations_and_order() {
        let config =
            Config::parse("version = 1\n[rules.subject-full-stop]\nseverity = 'warning'").unwrap();
        let input = format!(
            "# intro\nFeat: change.\n# before body\n{} \t\n# before footer\nRefs: {} \n",
            "x".repeat(73),
            "x".repeat(73)
        );
        let diagnostics = lint_filtered(&input, &config, "#", true, true);
        let actual: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.line, d.rule, d.severity))
            .collect();
        assert_eq!(
            actual,
            vec![
                (2, "subject-full-stop", Severity::Warning),
                (2, "type-case", Severity::Error),
                (4, "body-max-width", Severity::Error),
                (4, "control-character", Severity::Error),
                (4, "line-trailing-whitespace", Severity::Error),
                (4, "section-separator", Severity::Error),
                (6, "footer-max-width", Severity::Error),
                (6, "line-trailing-whitespace", Severity::Error),
                (6, "section-separator", Severity::Error),
            ]
        );
    }

    #[test]
    fn filtered_width_uses_configured_unicode_columns_and_severity() {
        let config = Config::parse(
            "version = 1\n[display-width]\nambiguous-width = 2\n\
             [rules.header-max-width]\nthresholds = [{limit = 8, severity = 'warning'}]",
        )
        .unwrap();
        let diagnostics = lint_filtered("# ignored\nfeat: “”\n", &config, "#", true, true);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            (
                diagnostics[0].line,
                diagnostics[0].rule,
                diagnostics[0].severity
            ),
            (2, "header-max-width", Severity::Warning)
        );
        assert_eq!(diagnostics[0].message, "display width 10 exceeds 8");
        assert!(lint_filtered("# ignored\nfeat: 👩‍💻\n", &config, "#", true, true).is_empty());
    }

    #[test]
    fn filtered_crlf_normalization_preserves_standalone_cr_and_original_positions() {
        let diagnostics =
            filtered_rules("# comment\r\nfeat: change\r\n\r\n# comment\r\nbody\r\nother\r");
        assert_eq!(
            diagnostics,
            vec![
                (6, "control-character", Severity::Error),
                (6, "line-trailing-whitespace", Severity::Error),
            ]
        );
    }

    #[test]
    fn parses_header_and_breaking_change() {
        assert!(rules("feat!: change API\n\nBREAKING CHANGE: reason\n").is_empty());
        assert!(rules("feat(api)!: change API").is_empty());
        assert_eq!(rules("feat:subject")[0].1, "header-format");
        assert!(rules("feat: ").iter().any(|d| d.1 == "subject-empty"));
        assert_eq!(rules("feat:  subject")[0].1, "header-format");
    }

    #[test]
    fn footer_candidate_and_separators_are_checked() {
        assert_eq!(
            rules("feat: change\n\nRefs:#123"),
            vec![(3, "trailer-format", Severity::Error)]
        );
        assert_eq!(
            rules("feat: change\nbody\nRefs: value"),
            vec![
                (2, "section-separator", Severity::Error),
                (3, "section-separator", Severity::Error),
            ]
        );
        assert!(rules("feat: change\n\nbody\n\nRefs: value\ncontinued").is_empty());
    }

    #[test]
    fn display_width_thresholds_use_columns() {
        let warning = format!("feat: {}", "あ".repeat(23));
        assert_eq!(
            rules(&warning),
            vec![(1, "header-max-width", Severity::Warning)]
        );
        let error = format!("feat: {}", "あ".repeat(34));
        assert_eq!(
            rules(&error),
            vec![(1, "header-max-width", Severity::Error)]
        );
    }

    #[test]
    fn crlf_matches_lf_and_controls_are_reported() {
        let lf = "feat: change\n\nbody \n";
        assert_eq!(rules(lf), rules(&lf.replace('\n', "\r\n")));
        assert!(
            rules("feat: change\u{202e}")
                .iter()
                .any(|d| d.1 == "control-character")
        );
        assert!(
            rules("feat: change\r")
                .iter()
                .any(|d| d.1 == "control-character")
        );
    }

    #[test]
    fn malformed_header_does_not_run_component_rules() {
        assert_eq!(
            rules("FIX:bad."),
            vec![(1, "header-format", Severity::Error)]
        );
    }

    #[test]
    fn malformed_trailer_like_lines_can_be_part_of_a_multiline_value() {
        assert!(rules("feat: change\n\nRefs: value\nRefs:#123\nmore").is_empty());
        assert!(rules("feat: change\n\nbody\nRefs:#123").is_empty());
    }

    #[test]
    fn trailer_tokens_can_contain_unicode_and_non_whitespace_characters() {
        assert!(rules("feat: change\n\nレビュー: 完了\nreviewed_by: name").is_empty());
        assert_eq!(
            rules("feat: change\n\nレビュー:値"),
            vec![(3, "trailer-format", Severity::Error)]
        );
    }

    #[test]
    fn body_and_footer_have_independent_width_rules() {
        let body = format!("feat: change\n\n{}", "x".repeat(73));
        assert_eq!(rules(&body), vec![(3, "body-max-width", Severity::Error)]);
        let footer = format!("feat: change\n\nRefs: {}", "x".repeat(68));
        assert_eq!(
            rules(&footer),
            vec![(3, "footer-max-width", Severity::Error)]
        );
    }

    #[test]
    fn scope_rules_match_unicode_and_allow_explicit_disabling() {
        let config = Config::parse(
            "version = 1\n[conventional]\nscope-required = true\nallowed-scopes = ['日本語']",
        )
        .unwrap();
        assert!(lint("feat(日本語): change", &config).is_empty());
        assert_eq!(lint("feat: change", &config)[0].rule, "scope-required");
        assert_eq!(lint("feat(日本): change", &config)[0].rule, "scope-enum");
    }

    #[test]
    fn unicode_width_data_and_string_algorithm_are_pinned() {
        assert_eq!(unicode_width::UNICODE_VERSION, (17, 0, 0));
        let config = Config::parse(
            "version = 1\n[display-width]\nambiguous-width = 2\n\
             [rules.header-max-width]\nthresholds = [{limit = 8, severity = 'error'}]",
        )
        .unwrap();
        assert_eq!(config.ambiguous_width, 2);
        assert_eq!("feat: “”".width_cjk(), 10);
        assert!(
            lint("feat: “”", &config)
                .iter()
                .any(|d| d.rule == "header-max-width")
        );
        assert!(lint("feat: 👩‍💻", &config).is_empty());
    }
}

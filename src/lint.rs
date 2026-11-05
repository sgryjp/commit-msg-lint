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

pub fn lint(input: &str, config: &Config) -> Vec<Diagnostic> {
    let normalized = input.replace("\r\n", "\n");
    let content = normalized.strip_suffix('\n').unwrap_or(&normalized);
    let lines: Vec<&str> = content.split('\n').collect();
    let mut linter = Linter {
        config,
        diagnostics: Vec::new(),
    };
    let header = lines[0];

    if header.trim() != header {
        linter.report(
            1,
            "header-trim",
            "header has leading or trailing whitespace",
        );
    }
    linter.width(1, "header-max-width", header);
    if let Some(parsed) = parse_header(header) {
        if !config
            .types
            .iter()
            .any(|kind| kind.eq_ignore_ascii_case(parsed.kind))
        {
            linter.report(1, "type-enum", "type is not in the allowed list");
        }
        if parsed.kind.bytes().any(|b| b.is_ascii_uppercase()) {
            linter.report(1, "type-case", "type must be lowercase");
        }
        if parsed.scope.is_none() {
            linter.report(1, "scope-required", "scope is required");
        }
        if let Some(scope) = parsed.scope
            && let Some(allowed) = &config.allowed_scopes
            && !allowed.iter().any(|value| value == scope)
        {
            linter.report(1, "scope-enum", "scope is not in the allowed list");
        }
        if parsed.subject.is_empty() {
            linter.report(1, "subject-empty", "subject must not be empty");
        }
        if parsed.subject.ends_with('.') {
            linter.report(
                1,
                "subject-full-stop",
                "subject must not end with a full stop",
            );
        }
    } else {
        linter.report(
            1,
            "header-format",
            "header does not match the Conventional Commit format",
        );
    }

    for (index, line) in lines.iter().enumerate() {
        if line.chars().any(forbidden_control) {
            linter.report(
                index + 1,
                "control-character",
                "line contains a forbidden control character",
            );
        }
        if index > 0 && line.trim_end() != *line {
            linter.report(
                index + 1,
                "line-trailing-whitespace",
                "line has trailing whitespace",
            );
        }
    }

    let first_content = (1..lines.len()).find(|&i| !lines[i].is_empty());
    if let Some(first) = first_content {
        if first != 2 {
            linter.report(
                first + 1,
                "section-separator",
                "header and following section must be separated by one empty line",
            );
        }
        let footer = (first..lines.len()).find(|&i| {
            valid_trailer(lines[i])
                || (trailer_candidate(lines[i]) && (i == first || lines[i - 1].is_empty()))
        });
        if let Some(start) = footer
            && start > first
        {
            let mut separator_start = start;
            while separator_start > first && lines[separator_start - 1].is_empty() {
                separator_start -= 1;
            }
            if start - separator_start != 1 {
                linter.report(
                    start + 1,
                    "section-separator",
                    "body and footer must be separated by one empty line",
                );
            }
        }
        let mut trailer_started = false;
        for (index, line) in lines.iter().enumerate().skip(first) {
            if footer.is_some_and(|start| index >= start) {
                linter.width(index + 1, "footer-max-width", line);
                if valid_trailer(line) {
                    trailer_started = true;
                } else if !trailer_started && index == footer.unwrap() {
                    linter.report(index + 1, "trailer-format", "invalid trailer");
                }
            } else {
                linter.width(index + 1, "body-max-width", line);
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

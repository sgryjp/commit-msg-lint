use std::collections::BTreeMap;
use std::fmt;

use jsonc_parser::ParseOptions;
use serde::{Deserialize, Serialize};

mod parser;

pub const CONFIG_SCHEMA_URL: &str =
    "https://sgryjp.github.io/commit-msg-lint/commit-msg-lint.schema.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigRange {
    pub start: usize,
    pub end: usize,
}

impl From<jsonc_parser::common::Range> for ConfigRange {
    fn from(range: jsonc_parser::common::Range) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum ConfigErrorCode {
    #[serde(rename = "config.syntax-error")]
    SyntaxError,
    #[serde(rename = "config.duplicate-key")]
    DuplicateKey,
    #[serde(rename = "config.missing-field")]
    MissingField,
    #[serde(rename = "config.unknown-field")]
    UnknownField,
    #[serde(rename = "config.unknown-rule")]
    UnknownRule,
    #[serde(rename = "config.type-mismatch")]
    TypeMismatch,
    #[serde(rename = "config.null-not-allowed")]
    NullNotAllowed,
    #[serde(rename = "config.invalid-enum")]
    InvalidEnum,
    #[serde(rename = "config.unsupported-version")]
    UnsupportedVersion,
    #[serde(rename = "config.out-of-range")]
    OutOfRange,
    #[serde(rename = "config.rule-setting-unsupported")]
    RuleSettingUnsupported,
    #[serde(rename = "config.rule-settings-conflict")]
    RuleSettingsConflict,
    #[serde(rename = "config.thresholds-empty")]
    ThresholdsEmpty,
    #[serde(rename = "config.threshold-limits-not-increasing")]
    ThresholdLimitsNotIncreasing,
    #[serde(rename = "config.threshold-severity-decreases")]
    ThresholdSeverityDecreases,
    #[serde(rename = "config.empty-types-while-type-enum-enabled")]
    EmptyTypesWhileTypeEnumEnabled,
    #[serde(rename = "config.scope-required-with-empty-allowed-scopes")]
    ScopeRequiredWithEmptyAllowedScopes,
    #[serde(rename = "config.scope-enum-without-allowed-scopes")]
    ScopeEnumWithoutAllowedScopes,
    #[serde(rename = "config.empty-header-pattern")]
    EmptyHeaderPattern,
    #[serde(rename = "config.invalid-header-pattern")]
    InvalidHeaderPattern,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigRelatedInformation {
    pub message: String,
    pub range: Option<ConfigRange>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigDiagnostic {
    pub code: ConfigErrorCode,
    pub message: String,
    pub range: Option<ConfigRange>,
    pub path: Option<String>,
    pub related: Vec<ConfigRelatedInformation>,
}

impl fmt::Display for ConfigDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigDiagnostic {}

const JSONC_OPTIONS: ParseOptions = ParseOptions {
    allow_comments: true,
    allow_loose_object_property_names: false,
    allow_trailing_commas: true,
    allow_missing_commas: false,
    allow_single_quoted_strings: false,
    allow_hexadecimal_numbers: false,
    allow_unary_plus_numbers: false,
    allow_bare_decimal_point_numbers: false,
    allow_non_finite_numbers: false,
    allow_extended_string_escapes: false,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => f.write_str("warning"),
            Self::Error => f.write_str("error"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    pub limit: usize,
    pub severity: Severity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleKind {
    Standard,
    Scope,
    HeaderWidth,
    LineWidth,
}

impl RuleKind {
    pub(crate) fn supports_severity(self) -> bool {
        match self {
            Self::Standard | Self::Scope | Self::LineWidth => true,
            Self::HeaderWidth => false,
        }
    }

    pub(crate) fn supports_thresholds(self) -> bool {
        match self {
            Self::HeaderWidth | Self::LineWidth => true,
            Self::Standard | Self::Scope => false,
        }
    }

    fn default_thresholds(self) -> &'static [Threshold] {
        match self {
            Self::HeaderWidth => &[
                Threshold {
                    limit: 50,
                    severity: Severity::Warning,
                },
                Threshold {
                    limit: 72,
                    severity: Severity::Error,
                },
            ],
            Self::LineWidth => &[Threshold {
                limit: 72,
                severity: Severity::Error,
            }],
            Self::Standard | Self::Scope => &[],
        }
    }
}

pub(crate) struct RuleMetadata {
    pub(crate) id: &'static str,
    pub(crate) kind: RuleKind,
    pub(crate) description: &'static str,
}

// Preserve the public Rule ID list without maintaining a second copy of the IDs.
macro_rules! define_rules {
    ($($id:literal => $kind:ident, $description:literal);+ $(;)?) => {
        pub const RULES: &[&str] = &[$($id),+];
        pub(crate) const RULE_METADATA: &[RuleMetadata] = &[
            $(RuleMetadata { id: $id, kind: RuleKind::$kind, description: $description }),+
        ];
    };
}

define_rules! {
    "header-format" => Standard, "Checks Header syntax: Type, optional Scope and !, and the colon-space separator.";
    "type-enum" => Standard, "Checks that the Type is in the allowed types list, ignoring ASCII case.";
    "type-case" => Standard, "Checks that the Type contains no uppercase ASCII letters.";
    "scope-required" => Scope, "Checks that a Scope is present.";
    "scope-enum" => Scope, "Checks that a present Scope matches an entry in allowed-scopes.";
    "subject-empty" => Standard, "Checks that the Subject is not empty.";
    "header-trim" => Standard, "Checks that the Header has no leading or trailing whitespace.";
    "subject-full-stop" => Standard, "Checks that the Subject does not end in a period.";
    "header-max-width" => HeaderWidth, "Checks Header width in Display Columns.";
    "section-separator" => Standard, "Checks that exactly one empty line separates Header, Body, and Footer.";
    "body-max-width" => LineWidth, "Checks the width of each Body line in Display Columns.";
    "footer-max-width" => LineWidth, "Checks the width of each Footer line in Display Columns.";
    "trailer-format" => Standard, "Checks that a Footer candidate starts with a valid Trailer.";
    "line-trailing-whitespace" => Standard, "Checks that Body and Footer lines have no trailing whitespace.";
    "control-character" => Standard, "Checks lines for forbidden control or bidirectional characters.";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub enabled: bool,
    pub severity: Severity,
    pub thresholds: Vec<Threshold>,
}

/// A nonempty, validated regex-lite pattern for matching a Header.
#[derive(Clone, Debug)]
pub struct HeaderPattern(regex_lite::Regex);

impl HeaderPattern {
    pub fn new(pattern: &str) -> Result<Self, String> {
        if pattern.is_empty() {
            return Err("ignore header-patterns must not contain an empty pattern".into());
        }
        regex_lite::Regex::new(pattern)
            .map(Self)
            .map_err(|error| format!("invalid ignore header-pattern {pattern:?}: {error}"))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn is_match(&self, header: &str) -> bool {
        self.0.is_match(header)
    }
}

impl PartialEq for HeaderPattern {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for HeaderPattern {}

impl Serialize for HeaderPattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HeaderPattern {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let pattern = String::deserialize(deserializer)?;
        Self::new(&pattern).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Ignore {
    pub merge: bool,
    pub revert: bool,
    pub autosquash: bool,
    pub header_patterns: Vec<HeaderPattern>,
}

impl Default for Ignore {
    fn default() -> Self {
        Self {
            merge: true,
            revert: true,
            autosquash: true,
            header_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub types: Vec<String>,
    pub allowed_scopes: Option<Vec<String>>,
    pub ambiguous_width: usize,
    pub ignore: Ignore,
    pub rules: BTreeMap<String, Rule>,
}

impl Default for Config {
    fn default() -> Self {
        let rules = RULE_METADATA
            .iter()
            .map(|metadata| {
                (
                    metadata.id.to_string(),
                    Rule {
                        enabled: metadata.kind != RuleKind::Scope,
                        severity: Severity::Error,
                        thresholds: metadata.kind.default_thresholds().to_vec(),
                    },
                )
            })
            .collect();
        Self {
            types: [
                "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert",
                "style", "test",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            allowed_scopes: None,
            ambiguous_width: 1,
            ignore: Ignore::default(),
            rules,
        }
    }
}

#[derive(Serialize)]
struct ConfigFile {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    version: u64,
    conventional: Conventional,
    #[serde(rename = "display-width")]
    display_width: DisplayWidth,
    ignore: Ignore,
    rules: BTreeMap<String, RuleOverride>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Conventional {
    #[serde(skip_serializing_if = "Option::is_none")]
    types: Option<Vec<String>>,
    scope_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_scopes: Option<Vec<String>>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct DisplayWidth {
    #[serde(skip_serializing_if = "Option::is_none")]
    ambiguous_width: Option<usize>,
}

#[derive(Serialize)]
struct RuleOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<Severity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thresholds: Option<Vec<Threshold>>,
}

impl Config {
    /// Returns a plain JSON representation of the built-in defaults.
    pub fn default_json() -> Result<String, serde_json::Error> {
        let config = Self::default();
        let rules = RULE_METADATA
            .iter()
            // Scope rules are controlled by the conventional settings, so emitting their
            // defaults as overrides would prevent edits to those settings from taking effect.
            .filter(|metadata| metadata.kind != RuleKind::Scope)
            .map(|metadata| {
                let rule = &config.rules[metadata.id];
                let has_thresholds = metadata.kind.supports_thresholds();
                (
                    metadata.id.to_string(),
                    RuleOverride {
                        enabled: Some(rule.enabled),
                        severity: (!has_thresholds).then_some(rule.severity),
                        thresholds: has_thresholds.then(|| rule.thresholds.clone()),
                    },
                )
            })
            .collect();
        serde_json::to_string_pretty(&ConfigFile {
            schema: Some(CONFIG_SCHEMA_URL.to_owned()),
            version: 1,
            conventional: Conventional {
                types: Some(config.types),
                scope_required: config.rules["scope-required"].enabled,
                allowed_scopes: config.allowed_scopes,
            },
            display_width: DisplayWidth {
                ambiguous_width: Some(config.ambiguous_width),
            },
            ignore: config.ignore,
            rules,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fields_report_a_stable_code_json_pointer_and_key_range() {
        let error = Config::parse(r#"{"version":1,"unexpected":true}"#).unwrap_err();
        assert_eq!(error.code, ConfigErrorCode::UnknownField);
        assert_eq!(error.path.as_deref(), Some("/unexpected"));
        assert_eq!(error.range, Some(ConfigRange { start: 13, end: 25 }));
        assert!(!error.message.is_empty());
        assert!(error.related.is_empty());
    }

    #[test]
    fn duplicate_decoded_keys_point_to_the_later_key_and_relate_the_first() {
        let input = r#"{"version":1,"version":2}"#;
        let error = Config::parse(input).unwrap_err();
        assert_eq!(error.code, ConfigErrorCode::DuplicateKey);
        assert_eq!(error.path.as_deref(), Some("/version"));
        assert_eq!(error.range, Some(ConfigRange { start: 13, end: 22 }));
        assert_eq!(error.related.len(), 1);
        assert_eq!(
            error.related[0].range,
            Some(ConfigRange { start: 1, end: 10 })
        );
        let related_json = serde_json::to_value(&error).unwrap()["related"][0].clone();
        assert_eq!(related_json.as_object().unwrap().len(), 3);
        assert_eq!(related_json["message"], "first occurrence of key: version");
        assert_eq!(related_json["path"], "/version");
        assert_eq!(related_json["range"]["start"], 1);

        let escaped = r#"{"version":1,"ver\u0073ion":2}"#;
        let escaped_error = Config::parse(escaped).unwrap_err();
        let later_key = escaped.find("\"ver\\u0073ion\"").unwrap();
        assert_eq!(escaped_error.code, ConfigErrorCode::DuplicateKey);
        assert_eq!(escaped_error.path.as_deref(), Some("/version"));
        assert_eq!(
            escaped_error.range,
            Some(ConfigRange {
                start: later_key,
                end: later_key + "\"ver\\u0073ion\"".len(),
            })
        );
        assert_eq!(
            escaped_error.related[0].range,
            Some(ConfigRange { start: 1, end: 10 })
        );

        let nested = r#"{"version":1,"rules":{"type-case":{"enabled":true,"enab\u006ced":false}}}"#;
        let nested_error = Config::parse(nested).unwrap_err();
        assert_eq!(nested_error.code, ConfigErrorCode::DuplicateKey);
        assert_eq!(
            nested_error.path.as_deref(),
            Some("/rules/type-case/enabled")
        );
        assert_eq!(nested_error.related.len(), 1);

        let thresholds = r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":1,"limit":2,"severity":"error"}]}}}"#;
        let threshold_error = Config::parse(thresholds).unwrap_err();
        assert_eq!(threshold_error.code, ConfigErrorCode::DuplicateKey);
        assert_eq!(
            threshold_error.path.as_deref(),
            Some("/rules/body-max-width/thresholds/0/limit")
        );
        assert_eq!(threshold_error.related.len(), 1);
    }

    #[test]
    fn syntax_and_missing_field_diagnostics_retain_eof_and_absent_positions() {
        let empty = Config::parse("").unwrap_err();
        assert_eq!(empty.code, ConfigErrorCode::SyntaxError);
        assert_eq!(empty.range, Some(ConfigRange { start: 0, end: 0 }));
        assert_eq!(empty.path, None);

        let unterminated = r#"{"version":1"#;
        let malformed = Config::parse(unterminated).unwrap_err();
        assert_eq!(malformed.code, ConfigErrorCode::SyntaxError);
        assert_eq!(
            malformed.range.unwrap().end,
            unterminated.len(),
            "unterminated input should report its EOF position"
        );
        assert_eq!(malformed.path, None);

        for json5 in [
            r#"{version:1}"#,
            r#"{"version":0x1}"#,
            r#"{"version":1,"conventional":{"types":['fix']}}"#,
        ] {
            let diagnostic = Config::parse(json5).unwrap_err();
            assert_eq!(diagnostic.code, ConfigErrorCode::SyntaxError, "{json5}");
            assert!(diagnostic.range.is_some(), "{json5}");
            assert_eq!(diagnostic.path, None, "{json5}");
        }

        let root_missing = Config::parse("{}").unwrap_err();
        assert_eq!(root_missing.code, ConfigErrorCode::MissingField);
        assert_eq!(root_missing.path.as_deref(), Some("/version"));
        assert_eq!(root_missing.range, None);

        let missing =
            Config::parse(r#"{"version":1,"rules":{"header-max-width":{"thresholds":[{}]}}}"#)
                .unwrap_err();
        assert_eq!(missing.code, ConfigErrorCode::MissingField);
        assert_eq!(
            missing.path.as_deref(),
            Some("/rules/header-max-width/thresholds/0/limit")
        );
        assert_eq!(missing.range, None);
    }

    #[test]
    fn threshold_and_rule_conflicts_report_primary_and_related_positions() {
        let input = r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":10,"severity":"warning"},{"limit":10,"severity":"error"}]}}}"#;
        let diagnostic = Config::parse(input).unwrap_err();
        let later_limit = input.rfind("10").unwrap();
        let previous_limit = input.find("10").unwrap();
        assert_eq!(
            diagnostic.code,
            ConfigErrorCode::ThresholdLimitsNotIncreasing
        );
        assert_eq!(
            diagnostic.path.as_deref(),
            Some("/rules/body-max-width/thresholds/1/limit")
        );
        assert_eq!(
            diagnostic.range,
            Some(ConfigRange {
                start: later_limit,
                end: later_limit + 2
            })
        );
        assert_eq!(
            diagnostic.related[0].range,
            Some(ConfigRange {
                start: previous_limit,
                end: previous_limit + 2
            })
        );

        let conflict_input = r#"{"version":1,"rules":{"body-max-width":{"severity":"warning","thresholds":[{"limit":10,"severity":"error"}]}}}"#;
        let conflict = Config::parse(conflict_input).unwrap_err();
        assert_eq!(conflict.code, ConfigErrorCode::RuleSettingsConflict);
        assert_eq!(
            conflict.path.as_deref(),
            Some("/rules/body-max-width/thresholds")
        );
        assert_eq!(
            conflict.related[0].path.as_deref(),
            Some("/rules/body-max-width/severity")
        );
        let thresholds_start = conflict_input.find("[{").unwrap();
        let thresholds_end = conflict_input.find("}]").unwrap() + 2;
        assert_eq!(
            conflict.range,
            Some(ConfigRange {
                start: thresholds_start,
                end: thresholds_end,
            })
        );
        let severity_start = conflict_input.find("\"warning\"").unwrap();
        assert_eq!(
            conflict.related[0].range,
            Some(ConfigRange {
                start: severity_start,
                end: severity_start + "\"warning\"".len(),
            })
        );

        let decreasing = Config::parse(
            r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":10,"severity":"error"},{"limit":20,"severity":"warning"}]}}}"#,
        )
        .unwrap_err();
        assert_eq!(decreasing.code, ConfigErrorCode::ThresholdSeverityDecreases);
        assert_eq!(
            decreasing.path.as_deref(),
            Some("/rules/body-max-width/thresholds/1/severity")
        );
        assert_eq!(
            decreasing.related[0].path.as_deref(),
            Some("/rules/body-max-width/thresholds/0/severity")
        );
        assert!(decreasing.range.is_some());
        assert!(decreasing.related[0].range.is_some());
        assert!(decreasing.related[0].message.contains("previous"));
    }

    #[test]
    fn each_published_configuration_failure_has_its_stable_code() {
        let cases = [
            ("", ConfigErrorCode::SyntaxError),
            (
                r#"{"version":1,"version":1}"#,
                ConfigErrorCode::DuplicateKey,
            ),
            (r#"{}"#, ConfigErrorCode::MissingField),
            (
                r#"{"version":1,"extra":true}"#,
                ConfigErrorCode::UnknownField,
            ),
            (
                r#"{"version":1,"rules":{"not-a-rule":{}}}"#,
                ConfigErrorCode::UnknownRule,
            ),
            (r#"{"version":"1"}"#, ConfigErrorCode::TypeMismatch),
            (r#"{"version":null}"#, ConfigErrorCode::NullNotAllowed),
            (
                r#"{"version":1,"rules":{"type-case":{"severity":"fatal"}}}"#,
                ConfigErrorCode::InvalidEnum,
            ),
            (r#"{"version":2}"#, ConfigErrorCode::UnsupportedVersion),
            (
                r#"{"version":1,"display-width":{"ambiguous-width":3}}"#,
                ConfigErrorCode::OutOfRange,
            ),
            (
                r#"{"version":1,"rules":{"header-max-width":{"severity":"error"}}}"#,
                ConfigErrorCode::RuleSettingUnsupported,
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"severity":"error","thresholds":[{"limit":10,"severity":"error"}]}}}"#,
                ConfigErrorCode::RuleSettingsConflict,
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"thresholds":[]}}}"#,
                ConfigErrorCode::ThresholdsEmpty,
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":10,"severity":"warning"},{"limit":10,"severity":"error"}]}}}"#,
                ConfigErrorCode::ThresholdLimitsNotIncreasing,
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":10,"severity":"error"},{"limit":20,"severity":"warning"}]}}}"#,
                ConfigErrorCode::ThresholdSeverityDecreases,
            ),
            (
                r#"{"version":1,"conventional":{"types":[]}}"#,
                ConfigErrorCode::EmptyTypesWhileTypeEnumEnabled,
            ),
            (
                r#"{"version":1,"conventional":{"scope-required":true,"allowed-scopes":[]}}"#,
                ConfigErrorCode::ScopeRequiredWithEmptyAllowedScopes,
            ),
            (
                r#"{"version":1,"rules":{"scope-enum":{"enabled":true}}}"#,
                ConfigErrorCode::ScopeEnumWithoutAllowedScopes,
            ),
            (
                r#"{"version":1,"ignore":{"header-patterns":[""]}}"#,
                ConfigErrorCode::EmptyHeaderPattern,
            ),
            (
                r#"{"version":1,"ignore":{"header-patterns":["["]}}"#,
                ConfigErrorCode::InvalidHeaderPattern,
            ),
        ];
        for (input, expected_code) in cases {
            let diagnostic = Config::parse(input).unwrap_err();
            assert_eq!(diagnostic.code, expected_code, "{input}");
            assert!(!diagnostic.message.is_empty(), "{input}");
        }
    }

    #[test]
    fn json_pointers_escape_keys_and_ranges_count_utf8_bytes_with_crlf() {
        let input = "{\r\n  \"version\": 1,\r\n  \"conventional\": {\"types\": [\"😀\"]},\r\n  \"a~/b\": true\r\n}";
        let error = Config::parse(input).unwrap_err();
        let key_start = input.find("\"a~/b\"").unwrap();
        assert_eq!(error.code, ConfigErrorCode::UnknownField);
        assert_eq!(error.path.as_deref(), Some("/a~0~1b"));
        assert_eq!(
            error.range,
            Some(ConfigRange {
                start: key_start,
                end: key_start + "\"a~/b\"".len(),
            })
        );

        let japanese =
            "{\r\n  \"version\": 1,\r\n  \"conventional\": {\"types\": [\"日本語😀\", 3]}\r\n}";
        let diagnostic = Config::parse(japanese).unwrap_err();
        let value_start = japanese.find(", 3").unwrap() + 2;
        assert_eq!(diagnostic.code, ConfigErrorCode::TypeMismatch);
        assert_eq!(diagnostic.path.as_deref(), Some("/conventional/types/1"));
        assert_eq!(
            diagnostic.range,
            Some(ConfigRange {
                start: value_start,
                end: value_start + 1,
            })
        );

        let related = r#"{"version":1,"設定":{"key":1,"key":2}}"#;
        let diagnostic = Config::parse(related).unwrap_err();
        assert_eq!(diagnostic.code, ConfigErrorCode::DuplicateKey);
        assert_eq!(diagnostic.path.as_deref(), Some("/設定/key"));
        assert_eq!(diagnostic.related[0].path.as_deref(), Some("/設定/key"));
    }

    #[test]
    fn typed_and_value_errors_report_their_json_pointer_and_value_range() {
        let cases = [
            (
                r#"{"version":"wrong"}"#,
                ConfigErrorCode::TypeMismatch,
                "/version",
                "\"wrong\"",
            ),
            (
                r#"{"version":1,"rules":{"typo":{}}}"#,
                ConfigErrorCode::UnknownRule,
                "/rules/typo",
                "\"typo\"",
            ),
            (
                r#"{"version":1,"display-width":{"ambiguous-width":"wide"}}"#,
                ConfigErrorCode::TypeMismatch,
                "/display-width/ambiguous-width",
                "\"wide\"",
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":false,"severity":"error"}]}}}"#,
                ConfigErrorCode::TypeMismatch,
                "/rules/body-max-width/thresholds/0/limit",
                "false",
            ),
            (
                r#"{"version":null}"#,
                ConfigErrorCode::NullNotAllowed,
                "/version",
                "null",
            ),
            (
                r#"{"version":1,"rules":{"type-case":{"severity":"fatal"}}}"#,
                ConfigErrorCode::InvalidEnum,
                "/rules/type-case/severity",
                "\"fatal\"",
            ),
            (
                r#"{"version":2}"#,
                ConfigErrorCode::UnsupportedVersion,
                "/version",
                "2",
            ),
            (
                r#"{"version":1,"display-width":{"ambiguous-width":3}}"#,
                ConfigErrorCode::OutOfRange,
                "/display-width/ambiguous-width",
                "3",
            ),
            (
                r#"{"version":1,"rules":{"header-max-width":{"severity":"error"}}}"#,
                ConfigErrorCode::RuleSettingUnsupported,
                "/rules/header-max-width/severity",
                "\"error\"",
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"thresholds":[]}}}"#,
                ConfigErrorCode::ThresholdsEmpty,
                "/rules/body-max-width/thresholds",
                "[]",
            ),
            (
                r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":10,"severity":"error"},{"limit":20,"severity":"warning"}]}}}"#,
                ConfigErrorCode::ThresholdSeverityDecreases,
                "/rules/body-max-width/thresholds/1/severity",
                "\"warning\"",
            ),
            (
                r#"{"version":1,"ignore":{"header-patterns":[""]}}"#,
                ConfigErrorCode::EmptyHeaderPattern,
                "/ignore/header-patterns/0",
                "\"\"",
            ),
            (
                r#"{"version":1,"ignore":{"header-patterns":["["]}}"#,
                ConfigErrorCode::InvalidHeaderPattern,
                "/ignore/header-patterns/0",
                "\"[\"",
            ),
        ];
        for (input, code, path, marker) in cases {
            let diagnostic = Config::parse(input).unwrap_err();
            let start = input.find(marker).unwrap();
            assert_eq!(diagnostic.code, code, "{input}");
            assert_eq!(diagnostic.path.as_deref(), Some(path), "{input}");
            assert_eq!(
                diagnostic.range,
                Some(ConfigRange {
                    start,
                    end: start + marker.len()
                }),
                "{input}"
            );
            assert!(!diagnostic.message.is_empty(), "{input}");
        }
    }

    #[test]
    fn related_settings_point_to_the_other_location_or_explain_defaults() {
        let explicit = Config::parse(
            r#"{"version":1,"conventional":{"types":[]},"rules":{"type-enum":{"enabled":true}}}"#,
        )
        .unwrap_err();
        assert_eq!(
            explicit.code,
            ConfigErrorCode::EmptyTypesWhileTypeEnumEnabled
        );
        assert_eq!(explicit.path.as_deref(), Some("/conventional/types"));
        assert_eq!(
            explicit.related[0].path.as_deref(),
            Some("/rules/type-enum/enabled")
        );
        assert!(explicit.related[0].range.is_some());

        let defaults = Config::parse(r#"{"version":1,"conventional":{"types":[]}}"#).unwrap_err();
        assert_eq!(
            defaults.code,
            ConfigErrorCode::EmptyTypesWhileTypeEnumEnabled
        );
        assert!(defaults.related[0].range.is_none());
        assert!(defaults.related[0].path.is_none());
        assert!(defaults.related[0].message.contains("by default"));

        let missing =
            Config::parse(r#"{"version":1,"rules":{"scope-enum":{"enabled":true}}}"#).unwrap_err();
        assert_eq!(
            missing.path.as_deref(),
            Some("/conventional/allowed-scopes")
        );
        assert!(missing.range.is_none());
        assert_eq!(
            missing.related[0].path.as_deref(),
            Some("/rules/scope-enum/enabled")
        );
        assert!(missing.related[0].range.is_some());

        let unsupported = Config::parse(r#"{"version":2}"#).unwrap_err();
        assert_eq!(unsupported.code, ConfigErrorCode::UnsupportedVersion);
        assert!(unsupported.related.is_empty());
    }

    #[test]
    fn scope_required_validation_uses_the_conventional_setting_before_rule_overrides() {
        let input = r#"{"version":1,"conventional":{"scope-required":true,"allowed-scopes":[]},"rules":{"scope-required":{"enabled":false}}}"#;
        let diagnostic = Config::parse(input).unwrap_err();
        assert_eq!(
            diagnostic.code,
            ConfigErrorCode::ScopeRequiredWithEmptyAllowedScopes
        );
        assert_eq!(
            diagnostic.path.as_deref(),
            Some("/conventional/allowed-scopes")
        );
        assert_eq!(
            diagnostic.related[0].path.as_deref(),
            Some("/conventional/scope-required")
        );
        assert!(diagnostic.range.is_some());
        assert!(diagnostic.related[0].range.is_some());
        assert!(diagnostic.related[0].message.contains("scope-required"));
    }

    #[test]
    fn diagnostic_json_includes_every_field_and_is_deterministic() {
        let input = r#"{"version":1,"first":true,"second":true}"#;
        let first = Config::parse(input).unwrap_err();
        let second = Config::parse(input).unwrap_err();
        assert_eq!(first, second);
        let json = serde_json::to_value(first).unwrap();
        assert_eq!(json["code"], "config.unknown-field");
        assert!(json["message"].is_string());
        assert!(json["range"]["start"].is_number());
        assert_eq!(json["path"], "/first");
        assert_eq!(json["related"], serde_json::json!([]));

        let root_type = Config::parse("[]").unwrap_err();
        assert_eq!(root_type.path.as_deref(), Some(""));
        assert!(root_type.related.is_empty());
        assert_eq!(
            serde_json::to_value(root_type).unwrap()["range"],
            serde_json::json!({"start": 0, "end": 2})
        );
    }

    #[test]
    fn each_rule_preserves_its_defaults_and_supported_settings() {
        let cases = [
            ("header-format", true, true, false),
            ("type-enum", true, true, false),
            ("type-case", true, true, false),
            ("scope-required", false, true, false),
            ("scope-enum", false, true, false),
            ("subject-empty", true, true, false),
            ("header-trim", true, true, false),
            ("subject-full-stop", true, true, false),
            ("header-max-width", true, false, true),
            ("section-separator", true, true, false),
            ("body-max-width", true, true, true),
            ("footer-max-width", true, true, true),
            ("trailer-format", true, true, false),
            ("line-trailing-whitespace", true, true, false),
            ("control-character", true, true, false),
        ];
        assert_eq!(RULES, cases.map(|(id, _, _, _)| id));
        let defaults = Config::default();
        assert_eq!(defaults.rules.len(), cases.len());
        for (id, default_enabled, supports_severity, supports_thresholds) in cases {
            let default = &defaults.rules[id];
            assert_eq!(default.enabled, default_enabled, "{id}");
            assert_eq!(default.severity, Severity::Error, "{id}");
            let expected_thresholds = match id {
                "header-max-width" => vec![
                    Threshold {
                        limit: 50,
                        severity: Severity::Warning,
                    },
                    Threshold {
                        limit: 72,
                        severity: Severity::Error,
                    },
                ],
                "body-max-width" | "footer-max-width" => vec![Threshold {
                    limit: 72,
                    severity: Severity::Error,
                }],
                _ => vec![],
            };
            assert_eq!(default.thresholds, expected_thresholds, "{id}");

            let parse = |settings: &str| {
                Config::parse(&format!(r#"{{"version":1,"rules":{{"{id}":{settings}}}}}"#))
            };
            let severity = parse(r#"{"enabled":false,"severity":"warning"}"#);
            if supports_severity {
                let rule = &severity.unwrap().rules[id];
                assert!(!rule.enabled, "{id}");
                assert_eq!(rule.severity, Severity::Warning, "{id}");
                assert!(
                    rule.thresholds
                        .iter()
                        .all(|t| t.severity == Severity::Warning),
                    "{id}"
                );
            } else {
                assert_eq!(
                    severity.unwrap_err().message,
                    format!("{id} uses thresholds, not severity")
                );
            }

            let thresholds =
                parse(r#"{"enabled":false,"thresholds":[{"limit":10,"severity":"warning"}]}"#);
            if supports_thresholds {
                let rule = &thresholds.unwrap().rules[id];
                assert!(!rule.enabled, "{id}");
                assert_eq!(
                    rule.thresholds,
                    [Threshold {
                        limit: 10,
                        severity: Severity::Warning
                    }],
                    "{id}"
                );
                assert_eq!(
                    parse(r#"{"thresholds":[]}"#).unwrap_err().code,
                    ConfigErrorCode::ThresholdsEmpty
                );
            } else {
                assert_eq!(
                    thresholds.unwrap_err().message,
                    format!("{id} does not support thresholds")
                );
            }

            let both =
                parse(r#"{"severity":"warning","thresholds":[{"limit":10,"severity":"warning"}]}"#);
            let expected_error = if !supports_severity {
                format!("{id} uses thresholds, not severity")
            } else if !supports_thresholds {
                format!("{id} does not support thresholds")
            } else {
                format!("{id} cannot specify both severity and thresholds")
            };
            assert_eq!(both.unwrap_err().message, expected_error);
        }
        assert_eq!(
            Config::parse(r#"{"version":1,"rules":{"unknown":{}}}"#)
                .unwrap_err()
                .message,
            "unknown rule: unknown"
        );
    }

    #[test]
    fn repository_configuration_preserves_its_allowed_types() {
        let config = Config::parse(include_str!("../commit-msg-lint.jsonc")).unwrap();
        assert_eq!(config.types, ["chore", "docs", "feat", "test"]);
    }

    #[test]
    fn generated_default_json_round_trips_all_default_settings() {
        let json = Config::default_json().unwrap();
        assert_eq!(Config::parse(&json).unwrap(), Config::default());
        assert!(json.starts_with("{\n"));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["$schema"], CONFIG_SCHEMA_URL);
        assert!(!json.contains("/*"));
        assert!(!json.contains(",\n  }"));
        assert!(!json.contains(",\n}"));
    }

    #[test]
    fn generated_defaults_leave_scope_rules_controlled_by_conventional_settings() {
        let json = Config::default_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["rules"].get("scope-required").is_none());
        assert!(value["rules"].get("scope-enum").is_none());
        let json = json.replace(
            "\"scope-required\": false",
            "\"scope-required\": true,\n    \"allowed-scopes\": [\"cli\"]",
        );
        let config = Config::parse(&json).unwrap();
        assert!(config.rules["scope-required"].enabled);
        assert!(config.rules["scope-enum"].enabled);
        assert_eq!(config.allowed_scopes, Some(vec!["cli".to_owned()]));
    }

    #[test]
    fn configuration_partially_overrides_profile() {
        let config = Config::parse(
            r#"
{
  "version": 1,
  "conventional": {
    "types": ["feat"],
    "scope-required": true,
    "allowed-scopes": ["cli"],
  },
  "rules": {
    "scope-required": { "enabled": false },
    "subject-full-stop": { "severity": "warning" },
    "header-max-width": {
      "thresholds": [
        { "limit": 10, "severity": "warning" },
        { "limit": 20, "severity": "error" },
      ],
    },
  },
}
// JSONC allows comments and trailing commas, but no other JSON5 syntax.
"#,
        )
        .unwrap();
        assert_eq!(config.types, ["feat"]);
        assert!(!config.rules["scope-required"].enabled);
        assert!(config.rules["scope-enum"].enabled);
        assert_eq!(
            config.rules["subject-full-stop"].severity,
            Severity::Warning
        );
        assert_eq!(config.rules["header-max-width"].thresholds[1].limit, 20);
        assert!(config.rules["type-case"].enabled);
    }

    #[test]
    fn accepts_both_comment_forms_and_comment_markers_inside_strings() {
        let config = Config::parse(
            r#"{
              // line comment
              "version": 1,
              "conventional": { "types": ["feat//api", "fix/*api*/"] },
              "rules": { "type-case": { "enabled": false } /* block comment */ },
            }"#,
        )
        .unwrap();
        assert_eq!(config.types, ["feat//api", "fix/*api*/"]);
        assert!(!config.rules["type-case"].enabled);
    }

    #[test]
    fn negative_zero_threshold_keeps_unsigned_integer_deserialization_behavior() {
        let config = Config::parse(
            r#"{"version":1,"rules":{"body-max-width":{"thresholds":[{"limit":-0,"severity":"error"}]}}}"#,
        )
        .unwrap();
        assert_eq!(config.rules["body-max-width"].thresholds[0].limit, 0);
    }

    #[test]
    fn omitted_optional_settings_preserve_their_defaults() {
        let config = Config::parse(r#"{ "version": 1 }"#).unwrap();
        let defaults = Config::default();
        assert_eq!(config.types, defaults.types);
        assert_eq!(config.allowed_scopes, defaults.allowed_scopes);
        assert_eq!(config.ambiguous_width, defaults.ambiguous_width);
        assert_eq!(config.rules["type-case"], defaults.rules["type-case"]);
        assert_eq!(
            config.rules["header-max-width"],
            defaults.rules["header-max-width"]
        );
    }

    #[test]
    fn rejects_null_in_every_optional_configuration_field() {
        for input in [
            r#"{"version":1,"$schema":null}"#,
            r#"{"version":1,"conventional":{"types":null}}"#,
            r#"{"version":1,"conventional":{"allowed-scopes":null}}"#,
            r#"{"version":1,"display-width":{"ambiguous-width":null}}"#,
            r#"{"version":1,"rules":{"type-case":{"enabled":null}}}"#,
            r#"{"version":1,"rules":{"type-case":{"severity":null}}}"#,
            r#"{"version":1,"rules":{"header-max-width":{"thresholds":null}}}"#,
            r#"{"version":1,"conventional":null}"#,
            r#"{"version":1,"display-width":null}"#,
            r#"{"version":1,"rules":null}"#,
        ] {
            assert!(Config::parse(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn rejects_duplicate_keys_in_structs_and_dynamic_rule_maps() {
        for input in [
            r#"{"version":1,"version":1}"#,
            r#"{"version":1,"ver\u0073ion":1}"#,
            r#"{"version":1,"rules":{"type-case":{"enabled":true,"enabled":false}}}"#,
            r#"{"version":1,"rules":{"type-case":{},"type-case":{}}}"#,
            r#"{"version":1,"rules":{"type-case":{},"type-\u0063ase":{}}}"#,
        ] {
            assert!(Config::parse(input).is_err(), "accepted duplicate {input}");
        }
    }

    #[test]
    fn rejects_invalid_jsonc_json5_extensions_unknown_keys_and_rule_relationships() {
        for input in [
            "",
            r#"{"version":2}"#,
            r#"{"version":1,"unexpected":true}"#,
            r#"{"version":1,"rules":{"unknown":{"enabled":true}}}"#,
            r#"{"version":1,"rules":{"type-case":{"thresholds":[]}}}"#,
            r#"{"version":1,"rules":{"type-case":{"severity":"fatal"}}}"#,
            r#"{"version":1,"display-width":{"ambiguous-width":3}}"#,
            r#"{"version":1,"conventional":{"types":[]}}"#,
            r#"{"version":1,"conventional":{"scope-required":true,"allowed-scopes":[]}}"#,
            r#"{"version":1,"rules":{"scope-enum":{"enabled":true}}}"#,
            r#"{"version":1,"rules":{"header-max-width":{"thresholds":[]}}}"#,
            r#"{"version":1,"rules":{"header-max-width":{"thresholds":[{"limit":10,"severity":"error"},{"limit":10,"severity":"error"}]}}}"#,
            r#"{"version":1,"rules":{"header-max-width":{"thresholds":[{"limit":10,"severity":"error"},{"limit":20,"severity":"warning"}]}}}"#,
            r#"{version:1}"#,
            r#"{'version':1}"#,
            r#"{"version":1,,}"#,
            r#"{"version":1 "conventional":{}}"#,
            r#"{"version":0x1}"#,
            r#"{"version":+1}"#,
            r#"{"version":.1}"#,
            r#"{"version":1.}"#,
            r#"{"version":Infinity}"#,
            r#"{"version":1,"conventional":{"types":['fix']}}"#,
        ] {
            assert!(Config::parse(input).is_err(), "accepted {input:?}");
        }
        assert!(Config::parse(
            r#"{"version":1,"conventional":{"types":[]},"rules":{"type-enum":{"enabled":false}}}"#
        )
        .is_ok());
    }
}

use std::collections::BTreeMap;
use std::fmt;

use jsonc_parser::{ParseOptions, parse_to_serde_value};
use serde::{Deserialize, Serialize};

pub const RULES: &[&str] = &[
    "header-format",
    "type-enum",
    "type-case",
    "scope-required",
    "scope-enum",
    "subject-empty",
    "header-trim",
    "subject-full-stop",
    "header-max-width",
    "section-separator",
    "body-max-width",
    "footer-max-width",
    "trailer-format",
    "line-trailing-whitespace",
    "control-character",
];

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

fn deserialize_non_null_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub enabled: bool,
    pub severity: Severity,
    pub thresholds: Vec<Threshold>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub types: Vec<String>,
    pub allowed_scopes: Option<Vec<String>>,
    pub ambiguous_width: usize,
    pub rules: BTreeMap<String, Rule>,
}

impl Default for Config {
    fn default() -> Self {
        let rules = RULES
            .iter()
            .map(|&id| {
                let thresholds = match id {
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
                    "body-max-width" | "footer-max-width" => {
                        vec![Threshold {
                            limit: 72,
                            severity: Severity::Error,
                        }]
                    }
                    _ => Vec::new(),
                };
                (
                    id.to_string(),
                    Rule {
                        enabled: !matches!(id, "scope-required" | "scope-enum"),
                        severity: Severity::Error,
                        thresholds,
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
            rules,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u64,
    #[serde(default)]
    conventional: Conventional,
    #[serde(default, rename = "display-width")]
    display_width: DisplayWidth,
    #[serde(default, with = "serde_with::rust::maps_duplicate_key_is_error")]
    rules: BTreeMap<String, RawRule>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct Conventional {
    #[serde(
        default,
        deserialize_with = "deserialize_non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    types: Option<Vec<String>>,
    scope_required: bool,
    #[serde(
        default,
        deserialize_with = "deserialize_non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    allowed_scopes: Option<Vec<String>>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct DisplayWidth {
    #[serde(
        default,
        deserialize_with = "deserialize_non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    ambiguous_width: Option<usize>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    #[serde(
        default,
        deserialize_with = "deserialize_non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    severity: Option<Severity>,
    #[serde(
        default,
        deserialize_with = "deserialize_non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    thresholds: Option<Vec<Threshold>>,
}

impl Config {
    /// Returns a plain JSON representation of the built-in defaults.
    pub fn default_json() -> Result<String, serde_json::Error> {
        let config = Self::default();
        let rules = config
            .rules
            .iter()
            // Scope rules are controlled by the conventional settings, so emitting their
            // defaults as overrides would prevent edits to those settings from taking effect.
            .filter(|(id, _)| !matches!(id.as_str(), "scope-required" | "scope-enum"))
            .map(|(id, rule)| {
                let has_thresholds = !rule.thresholds.is_empty();
                (
                    id.clone(),
                    RawRule {
                        enabled: Some(rule.enabled),
                        severity: (!has_thresholds).then_some(rule.severity),
                        thresholds: has_thresholds.then(|| rule.thresholds.clone()),
                    },
                )
            })
            .collect();
        serde_json::to_string_pretty(&RawConfig {
            version: 1,
            conventional: Conventional {
                types: Some(config.types),
                scope_required: config.rules["scope-required"].enabled,
                allowed_scopes: config.allowed_scopes,
            },
            display_width: DisplayWidth {
                ambiguous_width: Some(config.ambiguous_width),
            },
            rules,
        })
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let raw: RawConfig =
            parse_to_serde_value(input, &JSONC_OPTIONS).map_err(|e| e.to_string())?;
        if raw.version != 1 {
            return Err("version must be 1".into());
        }

        let mut config = Self::default();
        if let Some(types) = raw.conventional.types {
            config.types = types;
        }

        config.rules.get_mut("scope-required").unwrap().enabled = raw.conventional.scope_required;
        if let Some(scopes) = raw.conventional.allowed_scopes {
            config.rules.get_mut("scope-enum").unwrap().enabled = true;
            config.allowed_scopes = Some(scopes);
        }
        if let Some(width) = raw.display_width.ambiguous_width {
            if !matches!(width, 1 | 2) {
                return Err("ambiguous-width must be 1 or 2".into());
            }
            config.ambiguous_width = width;
        }
        for (id, override_rule) in raw.rules {
            let rule = config
                .rules
                .get_mut(&id)
                .ok_or_else(|| format!("unknown rule: {id}"))?;
            if let Some(enabled) = override_rule.enabled {
                rule.enabled = enabled;
            }
            if let Some(severity) = override_rule.severity {
                if id == "header-max-width" {
                    return Err(format!("{id} uses thresholds, not severity"));
                }
                rule.severity = severity;
                for threshold in &mut rule.thresholds {
                    threshold.severity = severity;
                }
            }
            if let Some(thresholds) = override_rule.thresholds {
                if rule.thresholds.is_empty() {
                    return Err(format!("{id} does not support thresholds"));
                }
                if override_rule.severity.is_some() {
                    return Err(format!("{id} cannot specify both severity and thresholds"));
                }
                if thresholds.is_empty()
                    || thresholds.windows(2).any(|pair| {
                        pair[0].limit >= pair[1].limit || pair[0].severity > pair[1].severity
                    })
                {
                    return Err(format!("{id} has invalid thresholds"));
                }
                rule.thresholds = thresholds;
            }
        }
        if config.rules["type-enum"].enabled && config.types.is_empty() {
            return Err("types must not be empty when type-enum is enabled".into());
        }
        if raw.conventional.scope_required
            && config.allowed_scopes.as_ref().is_some_and(Vec::is_empty)
        {
            return Err("scope-required cannot be combined with empty allowed-scopes".into());
        }
        if config.rules["scope-enum"].enabled && config.allowed_scopes.is_none() {
            return Err("scope-enum requires allowed-scopes".into());
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!json.contains("//"));
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

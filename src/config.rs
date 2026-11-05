use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    pub limit: usize,
    pub severity: Severity,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub enabled: bool,
    pub severity: Severity,
    pub thresholds: Vec<Threshold>,
}

#[derive(Debug)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u64,
    #[serde(default)]
    conventional: Conventional,
    #[serde(default, rename = "display-width")]
    display_width: DisplayWidth,
    #[serde(default)]
    rules: BTreeMap<String, RawRule>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct Conventional {
    types: Option<Vec<String>>,
    scope_required: bool,
    allowed_scopes: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct DisplayWidth {
    ambiguous_width: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    enabled: Option<bool>,
    severity: Option<Severity>,
    thresholds: Option<Vec<Threshold>>,
}

impl Config {
    pub fn parse(input: &str) -> Result<Self, String> {
        let raw: RawConfig = toml::from_str(input).map_err(|e| e.to_string())?;
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
    fn configuration_partially_overrides_profile() {
        let config = Config::parse(
            r#"
version = 1
[conventional]
types = ["feat"]
scope-required = true
allowed-scopes = ["cli"]
[rules.scope-required]
enabled = false
[rules.subject-full-stop]
severity = "warning"
[rules.header-max-width]
thresholds = [{limit = 10, severity = "warning"}, {limit = 20, severity = "error"}]
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
    fn rejects_invalid_keys_values_and_rule_relationships() {
        for input in [
            "",
            "version = 2",
            "version = 1\nunexpected = true",
            "version = 1\n[rules.unknown]\nenabled = true",
            "version = 1\n[rules.type-case]\nthresholds = []",
            "version = 1\n[rules.type-case]\nseverity = 'fatal'",
            "version = 1\n[display-width]\nambiguous-width = 3",
            "version = 1\n[conventional]\ntypes = []",
            "version = 1\n[conventional]\nscope-required = true\nallowed-scopes = []",
            "version = 1\n[rules.scope-enum]\nenabled = true",
            "version = 1\n[rules.header-max-width]\nthresholds = []",
            "version = 1\n[rules.header-max-width]\nthresholds = [{limit=10, severity='error'}, {limit=10, severity='error'}]",
            "version = 1\n[rules.header-max-width]\nthresholds = [{limit=10, severity='error'}, {limit=20, severity='warning'}]",
        ] {
            assert!(Config::parse(input).is_err(), "accepted {input:?}");
        }
        assert!(
            Config::parse(
                "version = 1\n[conventional]\ntypes = []\n[rules.type-enum]\nenabled = false"
            )
            .is_ok()
        );
    }
}

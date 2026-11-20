use std::collections::{BTreeMap, HashMap};
use std::fmt;

use jsonc_parser::ast::{Object, ObjectProp, Value};
use jsonc_parser::common::Ranged;
use jsonc_parser::{CollectOptions, parse_to_ast};
use serde::Serialize;

use super::{
    Config, Conventional, DisplayWidth, HeaderPattern, Ignore, JSONC_OPTIONS, RULE_METADATA,
    RawConfig, RawRule, Severity, Threshold,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceLocation {
    range: Option<ConfigRange>,
    path: Option<String>,
}

impl SourceLocation {
    fn new(range: Option<ConfigRange>, path: Option<String>) -> Self {
        Self { range, path }
    }

    fn at(range: impl Into<ConfigRange>, path: impl Into<String>) -> Self {
        Self::new(Some(range.into()), Some(path.into()))
    }

    fn missing(path: impl Into<String>) -> Self {
        Self::new(None, Some(path.into()))
    }

    fn unlocated() -> Self {
        Self::new(None, None)
    }
}

impl ConfigRelatedInformation {
    fn new(message: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            message: message.into(),
            range: location.range,
            path: location.path,
        }
    }
}

impl ConfigDiagnostic {
    fn new(code: ConfigErrorCode, message: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            code,
            message: message.into(),
            range: location.range,
            path: location.path,
            related: Vec::new(),
        }
    }

    fn with_related(mut self, related: ConfigRelatedInformation) -> Self {
        self.related.push(related);
        self
    }
}

impl fmt::Display for ConfigDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigDiagnostic {}

impl Config {
    /// Parses JSONC configuration and returns its first structured diagnostic on failure.
    pub fn parse(input: &str) -> Result<Self, ConfigDiagnostic> {
        let parsed =
            parse_to_ast(input, &CollectOptions::default(), &JSONC_OPTIONS).map_err(|error| {
                ConfigDiagnostic::new(
                    ConfigErrorCode::SyntaxError,
                    error.to_string(),
                    SourceLocation::new(Some(error.range().into()), None),
                )
            })?;
        let Some(value) = parsed.value else {
            let end = input.len();
            return Err(ConfigDiagnostic::new(
                ConfigErrorCode::SyntaxError,
                "expected a JSONC value",
                SourceLocation::new(Some(ConfigRange { start: end, end }), None),
            ));
        };

        validate_tree(&value, "")?;
        let root = expect_object(&value, "")?;
        let raw = parse_raw_config(root)?;
        build_config(raw, root)
    }
}

fn validate_tree(value: &Value<'_>, path: &str) -> Result<(), ConfigDiagnostic> {
    match value {
        Value::Object(object) => {
            let mut seen = HashMap::<String, ConfigRange>::new();
            for property in &object.properties {
                let key = property.name.as_str();
                let property_path = join_path(path, key);
                let key_range = property.name.range().into();
                if let Some(previous_range) = seen.get(key) {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::DuplicateKey,
                        format!("duplicate key: {key}"),
                        SourceLocation::at(key_range, property_path.clone()),
                    )
                    .with_related(ConfigRelatedInformation::new(
                        format!("first occurrence of key: {key}"),
                        SourceLocation::at(*previous_range, property_path),
                    )));
                }
                seen.insert(key.to_owned(), key_range);
                validate_tree(&property.value, &join_path(path, key))?;
            }
            Ok(())
        }
        Value::Array(array) => {
            for (index, element) in array.elements.iter().enumerate() {
                validate_tree(element, &join_path(path, &index.to_string()))?;
            }
            Ok(())
        }
        Value::NullKeyword(null) => Err(ConfigDiagnostic::new(
            ConfigErrorCode::NullNotAllowed,
            "null is not allowed; omit the field to use its default",
            SourceLocation::at(null.range, path),
        )),
        _ => Ok(()),
    }
}

fn parse_raw_config(root: &Object<'_>) -> Result<RawConfig, ConfigDiagnostic> {
    check_known_fields(
        root,
        "",
        &[
            "$schema",
            "version",
            "conventional",
            "display-width",
            "ignore",
            "rules",
        ],
    )?;

    let version = root
        .get("version")
        .ok_or_else(|| missing_field("version", "/version"))?;
    let version = expect_unsigned(&version.value, "/version")?;
    let schema = root
        .get("$schema")
        .map(|property| expect_string(&property.value, "/$schema").map(str::to_owned))
        .transpose()?;
    let conventional = parse_conventional(root.get("conventional"))?;
    let display_width = parse_display_width(root.get("display-width"))?;
    let ignore = parse_ignore(root.get("ignore"))?;
    let rules = parse_rules(root.get("rules"))?;

    Ok(RawConfig {
        schema,
        version,
        conventional,
        display_width,
        ignore,
        rules,
    })
}

fn parse_conventional(
    property: Option<&jsonc_parser::ast::ObjectProp<'_>>,
) -> Result<Conventional, ConfigDiagnostic> {
    let path = "/conventional";
    let Some(object) = optional_object(property, path)? else {
        return Ok(Conventional::default());
    };
    check_known_fields(object, path, &["types", "scope-required", "allowed-scopes"])?;
    let types = object
        .get("types")
        .map(|property| parse_string_array(&property.value, "/conventional/types"))
        .transpose()?;
    let scope_required = object
        .get("scope-required")
        .map(|property| expect_boolean(&property.value, "/conventional/scope-required"))
        .transpose()?
        .unwrap_or_default();
    let allowed_scopes = object
        .get("allowed-scopes")
        .map(|property| parse_string_array(&property.value, "/conventional/allowed-scopes"))
        .transpose()?;
    Ok(Conventional {
        types,
        scope_required,
        allowed_scopes,
    })
}

fn parse_display_width(
    property: Option<&jsonc_parser::ast::ObjectProp<'_>>,
) -> Result<DisplayWidth, ConfigDiagnostic> {
    let path = "/display-width";
    let Some(object) = optional_object(property, path)? else {
        return Ok(DisplayWidth::default());
    };
    check_known_fields(object, path, &["ambiguous-width"])?;
    let ambiguous_width = object
        .get("ambiguous-width")
        .map(|property| expect_usize(&property.value, "/display-width/ambiguous-width"))
        .transpose()?;
    Ok(DisplayWidth { ambiguous_width })
}

fn parse_ignore(
    property: Option<&jsonc_parser::ast::ObjectProp<'_>>,
) -> Result<Ignore, ConfigDiagnostic> {
    let path = "/ignore";
    let Some(object) = optional_object(property, path)? else {
        return Ok(Ignore::default());
    };
    check_known_fields(
        object,
        path,
        &["merge", "revert", "autosquash", "header-patterns"],
    )?;
    let mut ignore = Ignore::default();
    for (key, target) in [
        ("merge", &mut ignore.merge),
        ("revert", &mut ignore.revert),
        ("autosquash", &mut ignore.autosquash),
    ] {
        if let Some(property) = object.get(key) {
            *target = expect_boolean(&property.value, &join_path(path, key))?;
        }
    }
    if let Some(property) = object.get("header-patterns") {
        let patterns_path = "/ignore/header-patterns";
        let array = expect_array(&property.value, patterns_path)?;
        let mut patterns = Vec::with_capacity(array.elements.len());
        for (index, value) in array.elements.iter().enumerate() {
            let item_path = join_path(patterns_path, &index.to_string());
            let source = expect_string(value, &item_path)?;
            if source.is_empty() {
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::EmptyHeaderPattern,
                    "ignore header-patterns must not contain an empty pattern",
                    SourceLocation::at(value.range(), item_path),
                ));
            }
            let pattern = HeaderPattern::new(source).map_err(|message| {
                ConfigDiagnostic::new(
                    ConfigErrorCode::InvalidHeaderPattern,
                    message,
                    SourceLocation::at(value.range(), item_path),
                )
            })?;
            patterns.push(pattern);
        }
        ignore.header_patterns = patterns;
    }
    Ok(ignore)
}

fn parse_rules(
    property: Option<&jsonc_parser::ast::ObjectProp<'_>>,
) -> Result<BTreeMap<String, RawRule>, ConfigDiagnostic> {
    let path = "/rules";
    let Some(object) = optional_object(property, path)? else {
        return Ok(BTreeMap::new());
    };
    let mut rules = BTreeMap::new();
    for property in &object.properties {
        let id = property.name.as_str();
        let rule_path = join_path(path, id);
        if !RULE_METADATA.iter().any(|metadata| metadata.id == id) {
            return Err(ConfigDiagnostic::new(
                ConfigErrorCode::UnknownRule,
                format!("unknown rule: {id}"),
                SourceLocation::at(property.name.range(), rule_path),
            ));
        }
        let object = expect_object(&property.value, &rule_path)?;
        check_known_fields(object, &rule_path, &["enabled", "severity", "thresholds"])?;
        let enabled = object
            .get("enabled")
            .map(|property| expect_boolean(&property.value, &join_path(&rule_path, "enabled")))
            .transpose()?;
        let severity = object
            .get("severity")
            .map(|property| parse_severity(&property.value, &join_path(&rule_path, "severity")))
            .transpose()?;
        let thresholds = object
            .get("thresholds")
            .map(|property| parse_thresholds(&property.value, &join_path(&rule_path, "thresholds")))
            .transpose()?;
        rules.insert(
            id.to_owned(),
            RawRule {
                enabled,
                severity,
                thresholds,
            },
        );
    }
    Ok(rules)
}

fn parse_thresholds(value: &Value<'_>, path: &str) -> Result<Vec<Threshold>, ConfigDiagnostic> {
    let array = expect_array(value, path)?;
    array
        .elements
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let item_path = join_path(path, &index.to_string());
            let object = expect_object(value, &item_path)?;
            check_known_fields(object, &item_path, &["limit", "severity"])?;
            let limit = object
                .get("limit")
                .ok_or_else(|| missing_field("limit", &join_path(&item_path, "limit")))?;
            let severity = object
                .get("severity")
                .ok_or_else(|| missing_field("severity", &join_path(&item_path, "severity")))?;
            Ok(Threshold {
                limit: expect_usize(&limit.value, &join_path(&item_path, "limit"))?,
                severity: parse_severity(&severity.value, &join_path(&item_path, "severity"))?,
            })
        })
        .collect()
}

fn build_config(raw: RawConfig, root: &Object<'_>) -> Result<Config, ConfigDiagnostic> {
    if raw.version != 1 {
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::UnsupportedVersion,
            format!("unsupported configuration version: {}", raw.version),
            location_at(root, &["version"]).expect("parsed config has a version location"),
        ));
    }

    let conventional_scope_required = raw.conventional.scope_required;
    let mut config = Config {
        ignore: raw.ignore,
        ..Config::default()
    };
    if let Some(types) = raw.conventional.types {
        config.types = types;
    }

    config.rules.get_mut("scope-required").unwrap().enabled = conventional_scope_required;
    if let Some(scopes) = raw.conventional.allowed_scopes {
        config.rules.get_mut("scope-enum").unwrap().enabled = true;
        config.allowed_scopes = Some(scopes);
    }
    if let Some(width) = raw.display_width.ambiguous_width {
        if !matches!(width, 1 | 2) {
            let location = location_at(root, &["display-width", "ambiguous-width"])
                .expect("parsed ambiguous-width has a source location");
            return Err(ConfigDiagnostic::new(
                ConfigErrorCode::OutOfRange,
                "ambiguous-width must be 1 or 2",
                location,
            ));
        }
        config.ambiguous_width = width;
    }

    for (id, override_rule) in raw.rules {
        let metadata = RULE_METADATA
            .iter()
            .find(|metadata| metadata.id == id)
            .expect("parsed Rule IDs are known");
        let path = join_path("/rules", &id);
        let source = property_at(root, &["rules", id.as_str()])
            .and_then(|property| property.value.as_object())
            .expect("parsed Rule settings have source locations");
        let rule = config.rules.get_mut(&id).unwrap();
        if let Some(enabled) = override_rule.enabled {
            rule.enabled = enabled;
        }
        if let Some(severity) = override_rule.severity {
            if !metadata.kind.supports_severity() {
                let property = source.get("severity").unwrap();
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::RuleSettingUnsupported,
                    format!("{id} uses thresholds, not severity"),
                    SourceLocation::at(property.value.range(), join_path(&path, "severity")),
                ));
            }
            rule.severity = severity;
            for threshold in &mut rule.thresholds {
                threshold.severity = severity;
            }
        }
        if let Some(thresholds) = override_rule.thresholds {
            if !metadata.kind.supports_thresholds() {
                let property = source.get("thresholds").unwrap();
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::RuleSettingUnsupported,
                    format!("{id} does not support thresholds"),
                    SourceLocation::at(property.value.range(), join_path(&path, "thresholds")),
                ));
            }
            if override_rule.severity.is_some() {
                let thresholds_property = source.get("thresholds").unwrap();
                let severity_property = source.get("severity").unwrap();
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::RuleSettingsConflict,
                    format!("{id} cannot specify both severity and thresholds"),
                    SourceLocation::at(
                        thresholds_property.value.range(),
                        join_path(&path, "thresholds"),
                    ),
                )
                .with_related(ConfigRelatedInformation::new(
                    "severity is also specified here",
                    SourceLocation::at(
                        severity_property.value.range(),
                        join_path(&path, "severity"),
                    ),
                )));
            }
            if thresholds.is_empty() {
                let property = source.get("thresholds").unwrap();
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::ThresholdsEmpty,
                    format!("{id} thresholds must not be empty"),
                    SourceLocation::at(property.value.range(), join_path(&path, "thresholds")),
                ));
            }
            let threshold_array = source
                .get("thresholds")
                .and_then(|property| property.value.as_array())
                .expect("parsed thresholds have source locations");
            let thresholds_path = join_path(&path, "thresholds");
            for (index, pair) in thresholds.windows(2).enumerate() {
                let earlier_path = join_path(&thresholds_path, &index.to_string());
                let later_path = join_path(&thresholds_path, &(index + 1).to_string());
                let earlier = threshold_array.elements[index]
                    .as_object()
                    .expect("parsed threshold entries have source locations");
                let later = threshold_array.elements[index + 1]
                    .as_object()
                    .expect("parsed threshold entries have source locations");
                if pair[0].limit >= pair[1].limit {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::ThresholdLimitsNotIncreasing,
                        format!("{id} threshold limits must be strictly increasing"),
                        SourceLocation::at(
                            later.get("limit").unwrap().value.range(),
                            join_path(&later_path, "limit"),
                        ),
                    )
                    .with_related(ConfigRelatedInformation::new(
                        "previous threshold limit",
                        SourceLocation::at(
                            earlier.get("limit").unwrap().value.range(),
                            join_path(&earlier_path, "limit"),
                        ),
                    )));
                }
                if pair[0].severity > pair[1].severity {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::ThresholdSeverityDecreases,
                        format!("{id} threshold severity must not decrease"),
                        SourceLocation::at(
                            later.get("severity").unwrap().value.range(),
                            join_path(&later_path, "severity"),
                        ),
                    )
                    .with_related(ConfigRelatedInformation::new(
                        "previous threshold severity",
                        SourceLocation::at(
                            earlier.get("severity").unwrap().value.range(),
                            join_path(&earlier_path, "severity"),
                        ),
                    )));
                }
            }
            rule.thresholds = thresholds;
        }
    }

    if config.rules["type-enum"].enabled && config.types.is_empty() {
        let types_location = location_at(root, &["conventional", "types"])
            .expect("empty types must be explicitly configured");
        let type_enum_enabled = location_at(root, &["rules", "type-enum", "enabled"]);
        let related = match type_enum_enabled {
            Some(location) => ConfigRelatedInformation::new("type-enum is enabled here", location),
            None => ConfigRelatedInformation::new(
                "type-enum is enabled by default",
                SourceLocation::unlocated(),
            ),
        };
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::EmptyTypesWhileTypeEnumEnabled,
            "types must not be empty when type-enum is enabled",
            types_location,
        )
        .with_related(related));
    }

    if conventional_scope_required && config.allowed_scopes.as_ref().is_some_and(Vec::is_empty) {
        let scopes_location = location_at(root, &["conventional", "allowed-scopes"])
            .expect("empty allowed-scopes must be explicitly configured");
        let scope_required_location = location_at(root, &["conventional", "scope-required"])
            .expect("scope-required true must be explicitly configured");
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::ScopeRequiredWithEmptyAllowedScopes,
            "scope-required cannot be combined with empty allowed-scopes",
            scopes_location,
        )
        .with_related(ConfigRelatedInformation::new(
            "scope-required is enabled here",
            scope_required_location,
        )));
    }

    if config.rules["scope-enum"].enabled && config.allowed_scopes.is_none() {
        let scope_enum_enabled = location_at(root, &["rules", "scope-enum", "enabled"])
            .expect("scope-enum cannot be enabled without an explicit override");
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::ScopeEnumWithoutAllowedScopes,
            "scope-enum requires allowed-scopes",
            SourceLocation::missing(pointer(&["conventional", "allowed-scopes"])),
        )
        .with_related(ConfigRelatedInformation::new(
            "scope-enum is enabled here",
            scope_enum_enabled,
        )));
    }

    Ok(config)
}

fn optional_object<'node, 'text>(
    property: Option<&'node ObjectProp<'text>>,
    path: &str,
) -> Result<Option<&'node Object<'text>>, ConfigDiagnostic> {
    property
        .map(|property| expect_object(&property.value, path))
        .transpose()
}

fn expect_object<'node, 'text>(
    value: &'node Value<'text>,
    path: &str,
) -> Result<&'node Object<'text>, ConfigDiagnostic> {
    value
        .as_object()
        .ok_or_else(|| type_mismatch(value, path, "an object"))
}

fn expect_array<'node, 'text>(
    value: &'node Value<'text>,
    path: &str,
) -> Result<&'node jsonc_parser::ast::Array<'text>, ConfigDiagnostic> {
    value
        .as_array()
        .ok_or_else(|| type_mismatch(value, path, "an array"))
}

fn expect_string<'a>(value: &'a Value<'_>, path: &str) -> Result<&'a str, ConfigDiagnostic> {
    value
        .as_string_lit()
        .map(|literal| literal.value.as_ref())
        .ok_or_else(|| type_mismatch(value, path, "a string"))
}

fn expect_boolean(value: &Value<'_>, path: &str) -> Result<bool, ConfigDiagnostic> {
    value
        .as_boolean_lit()
        .map(|literal| literal.value)
        .ok_or_else(|| type_mismatch(value, path, "a boolean"))
}

fn expect_unsigned(value: &Value<'_>, path: &str) -> Result<u64, ConfigDiagnostic> {
    let Some(number) = value.as_number_lit() else {
        return Err(type_mismatch(value, path, "an unsigned integer"));
    };
    let parsed = if number.value == "-0" {
        Some(0)
    } else if number.value.starts_with('-') {
        None
    } else {
        number.value.parse::<u64>().ok()
    };
    match parsed {
        Some(number) => Ok(number),
        None => Err(ConfigDiagnostic::new(
            ConfigErrorCode::TypeMismatch,
            "expected an unsigned integer in range",
            SourceLocation::at(number.range, path),
        )),
    }
}

fn expect_usize(value: &Value<'_>, path: &str) -> Result<usize, ConfigDiagnostic> {
    let literal = value
        .as_number_lit()
        .expect("expect_unsigned requires a number literal");
    usize::try_from(expect_unsigned(value, path)?).map_err(|_| {
        ConfigDiagnostic::new(
            ConfigErrorCode::TypeMismatch,
            "expected an unsigned integer in range",
            SourceLocation::at(literal.range, path),
        )
    })
}

fn parse_string_array(value: &Value<'_>, path: &str) -> Result<Vec<String>, ConfigDiagnostic> {
    let array = expect_array(value, path)?;
    array
        .elements
        .iter()
        .enumerate()
        .map(|(index, value)| {
            expect_string(value, &join_path(path, &index.to_string())).map(str::to_owned)
        })
        .collect()
}

fn parse_severity(value: &Value<'_>, path: &str) -> Result<Severity, ConfigDiagnostic> {
    match expect_string(value, path)? {
        "warning" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        name => Err(ConfigDiagnostic::new(
            ConfigErrorCode::InvalidEnum,
            format!("invalid severity: {name}"),
            SourceLocation::at(value.range(), path),
        )),
    }
}

fn check_known_fields(
    object: &Object<'_>,
    path: &str,
    allowed: &[&str],
) -> Result<(), ConfigDiagnostic> {
    for property in &object.properties {
        let key = property.name.as_str();
        if !allowed.contains(&key) {
            return Err(ConfigDiagnostic::new(
                ConfigErrorCode::UnknownField,
                format!("unknown field: {key}"),
                SourceLocation::at(property.name.range(), join_path(path, key)),
            ));
        }
    }
    Ok(())
}

fn location_at(root: &Object<'_>, components: &[&str]) -> Option<SourceLocation> {
    let property = property_at(root, components)?;
    Some(SourceLocation::at(
        property.value.range(),
        pointer(components),
    ))
}

fn pointer(components: &[&str]) -> String {
    components
        .iter()
        .fold(String::new(), |path, component| join_path(&path, component))
}

fn property_at<'node, 'text>(
    root: &'node Object<'text>,
    components: &[&str],
) -> Option<&'node ObjectProp<'text>> {
    let mut object = root;
    for (index, component) in components.iter().enumerate() {
        let property = object.get(component)?;
        if index + 1 == components.len() {
            return Some(property);
        }
        object = property.value.as_object()?;
    }
    None
}

fn missing_field(name: &str, path: &str) -> ConfigDiagnostic {
    ConfigDiagnostic::new(
        ConfigErrorCode::MissingField,
        format!("missing field: {name}"),
        SourceLocation::missing(path),
    )
}

fn type_mismatch(value: &Value<'_>, path: &str, expected: &str) -> ConfigDiagnostic {
    ConfigDiagnostic::new(
        ConfigErrorCode::TypeMismatch,
        format!("expected {expected}"),
        SourceLocation::at(value.range(), path),
    )
}

fn join_path(parent: &str, component: &str) -> String {
    let escaped = component.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

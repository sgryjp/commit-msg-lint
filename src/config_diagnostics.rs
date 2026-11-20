use std::collections::HashMap;

use jsonc_parser::ast::{Object, Value};
use jsonc_parser::common::Ranged;
use jsonc_parser::{CollectOptions, parse_to_ast};

use super::{
    Config, ConfigDiagnostic, ConfigErrorCode, ConfigRange, HeaderPattern, JSONC_OPTIONS,
    RULE_METADATA, RawConfig,
};

pub(super) fn parse(input: &str) -> Result<Config, ConfigDiagnostic> {
    let parsed =
        parse_to_ast(input, &CollectOptions::default(), &JSONC_OPTIONS).map_err(|error| {
            ConfigDiagnostic::new(
                ConfigErrorCode::SyntaxError,
                error.to_string(),
                Some(error.range().into()),
                None,
            )
        })?;
    let Some(value) = parsed.value else {
        let end = input.len();
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::SyntaxError,
            "expected a JSONC value",
            Some(ConfigRange { start: end, end }),
            None,
        ));
    };

    validate_duplicate_keys(&value, "")?;
    validate_no_nulls(&value, "")?;
    let root = expect_object(&value, "")?;
    validate_config_shape(root)?;

    let json: serde_json::Value = value.clone().into();
    let raw: RawConfig = serde_json::from_value(json).map_err(|error| {
        ConfigDiagnostic::new(ConfigErrorCode::TypeMismatch, error.to_string(), None, None)
    })?;
    build_config(raw, root)
}

fn validate_duplicate_keys(value: &Value<'_>, path: &str) -> Result<(), ConfigDiagnostic> {
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
                        Some(key_range),
                        Some(property_path.clone()),
                    )
                    .with_related(
                        format!("first occurrence of key: {key}"),
                        Some(*previous_range),
                        Some(property_path),
                    ));
                }
                validate_duplicate_keys(&property.value, &property_path)?;
                seen.insert(key.to_owned(), key_range);
            }
        }
        Value::Array(array) => {
            for (index, element) in array.elements.iter().enumerate() {
                validate_duplicate_keys(element, &join_path(path, &index.to_string()))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_no_nulls(value: &Value<'_>, path: &str) -> Result<(), ConfigDiagnostic> {
    match value {
        Value::NullKeyword(null) => Err(ConfigDiagnostic::new(
            ConfigErrorCode::NullNotAllowed,
            "null is not allowed; omit the field to use its default",
            Some(null.range.into()),
            Some(path.to_owned()),
        )),
        Value::Object(object) => {
            for property in &object.properties {
                validate_no_nulls(&property.value, &join_path(path, property.name.as_str()))?;
            }
            Ok(())
        }
        Value::Array(array) => {
            for (index, element) in array.elements.iter().enumerate() {
                validate_no_nulls(element, &join_path(path, &index.to_string()))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_config_shape(root: &Object<'_>) -> Result<(), ConfigDiagnostic> {
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
    expect_unsigned(&version.value, "/version", u64::MAX)?;
    if let Some(schema) = root.get("$schema") {
        expect_string(&schema.value, "/$schema")?;
    }
    if let Some(conventional) = root.get("conventional") {
        let object = expect_object(&conventional.value, "/conventional")?;
        check_known_fields(
            object,
            "/conventional",
            &["types", "scope-required", "allowed-scopes"],
        )?;
        if let Some(types) = object.get("types") {
            validate_string_array(&types.value, "/conventional/types")?;
        }
        if let Some(scope_required) = object.get("scope-required") {
            expect_boolean(&scope_required.value, "/conventional/scope-required")?;
        }
        if let Some(scopes) = object.get("allowed-scopes") {
            validate_string_array(&scopes.value, "/conventional/allowed-scopes")?;
        }
    }
    if let Some(display_width) = root.get("display-width") {
        let object = expect_object(&display_width.value, "/display-width")?;
        check_known_fields(object, "/display-width", &["ambiguous-width"])?;
        if let Some(width) = object.get("ambiguous-width") {
            expect_unsigned(
                &width.value,
                "/display-width/ambiguous-width",
                usize::MAX as u64,
            )?;
        }
    }
    if let Some(ignore) = root.get("ignore") {
        let object = expect_object(&ignore.value, "/ignore")?;
        check_known_fields(
            object,
            "/ignore",
            &["merge", "revert", "autosquash", "header-patterns"],
        )?;
        for key in ["merge", "revert", "autosquash"] {
            if let Some(value) = object.get(key) {
                expect_boolean(&value.value, &join_path("/ignore", key))?;
            }
        }
        if let Some(patterns) = object.get("header-patterns") {
            let path = "/ignore/header-patterns";
            let array = expect_array(&patterns.value, path)?;
            for (index, value) in array.elements.iter().enumerate() {
                let item_path = join_path(path, &index.to_string());
                let pattern = expect_string(value, &item_path)?;
                if pattern.is_empty() {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::EmptyHeaderPattern,
                        "ignore header-patterns must not contain an empty pattern",
                        Some(value.range().into()),
                        Some(item_path),
                    ));
                }
                if let Err(error) = HeaderPattern::new(pattern) {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::InvalidHeaderPattern,
                        error,
                        Some(value.range().into()),
                        Some(item_path),
                    ));
                }
            }
        }
    }
    if let Some(rules) = root.get("rules") {
        let object = expect_object(&rules.value, "/rules")?;
        for property in &object.properties {
            let id = property.name.as_str();
            let path = join_path("/rules", id);
            let Some(_metadata) = RULE_METADATA.iter().find(|metadata| metadata.id == id) else {
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::UnknownRule,
                    format!("unknown rule: {id}"),
                    Some(property.name.range().into()),
                    Some(path),
                ));
            };
            let rule = expect_object(&property.value, &path)?;
            check_known_fields(rule, &path, &["enabled", "severity", "thresholds"])?;
            if let Some(enabled) = rule.get("enabled") {
                expect_boolean(&enabled.value, &join_path(&path, "enabled"))?;
            }
            if let Some(severity) = rule.get("severity") {
                validate_severity(&severity.value, &join_path(&path, "severity"))?;
            }
            if let Some(thresholds) = rule.get("thresholds") {
                let thresholds_path = join_path(&path, "thresholds");
                let array = expect_array(&thresholds.value, &thresholds_path)?;
                for (index, threshold) in array.elements.iter().enumerate() {
                    let threshold_path = join_path(&thresholds_path, &index.to_string());
                    let threshold = expect_object(threshold, &threshold_path)?;
                    check_known_fields(threshold, &threshold_path, &["limit", "severity"])?;
                    let limit = threshold.get("limit").ok_or_else(|| {
                        missing_field("limit", &join_path(&threshold_path, "limit"))
                    })?;
                    expect_unsigned(
                        &limit.value,
                        &join_path(&threshold_path, "limit"),
                        usize::MAX as u64,
                    )?;
                    let severity = threshold.get("severity").ok_or_else(|| {
                        missing_field("severity", &join_path(&threshold_path, "severity"))
                    })?;
                    validate_severity(&severity.value, &join_path(&threshold_path, "severity"))?;
                }
            }
        }
    }
    Ok(())
}

fn build_config(raw: RawConfig, root: &Object<'_>) -> Result<Config, ConfigDiagnostic> {
    let version = root
        .get("version")
        .expect("shape validation requires version");
    if raw.version != 1 {
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::UnsupportedVersion,
            format!("unsupported configuration version: {}", raw.version),
            Some(version.value.range().into()),
            Some("/version".to_owned()),
        ));
    }

    let mut config = Config {
        ignore: raw.ignore,
        ..Config::default()
    };
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
            let property = root
                .get("display-width")
                .and_then(|value| value.value.as_object())
                .and_then(|object| object.get("ambiguous-width"))
                .expect("shape validation established ambiguous-width");
            return Err(ConfigDiagnostic::new(
                ConfigErrorCode::OutOfRange,
                "ambiguous-width must be 1 or 2",
                Some(property.value.range().into()),
                Some("/display-width/ambiguous-width".to_owned()),
            ));
        }
        config.ambiguous_width = width;
    }

    let rules_object = root.get("rules").and_then(|value| value.value.as_object());
    for (id, override_rule) in raw.rules {
        let metadata = RULE_METADATA
            .iter()
            .find(|metadata| metadata.id == id)
            .expect("shape validation established known rule IDs");
        let path = join_path("/rules", &id);
        let source = rules_object
            .and_then(|rules| rules.get(&id))
            .and_then(|value| value.value.as_object())
            .expect("shape validation established rule objects");
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
                    Some(property.value.range().into()),
                    Some(join_path(&path, "severity")),
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
                    Some(property.value.range().into()),
                    Some(join_path(&path, "thresholds")),
                ));
            }
            if override_rule.severity.is_some() {
                let thresholds_property = source.get("thresholds").unwrap();
                let severity_property = source.get("severity").unwrap();
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::RuleSettingsConflict,
                    format!("{id} cannot specify both severity and thresholds"),
                    Some(thresholds_property.value.range().into()),
                    Some(join_path(&path, "thresholds")),
                )
                .with_related(
                    "severity is also specified here",
                    Some(severity_property.value.range().into()),
                    Some(join_path(&path, "severity")),
                ));
            }
            if thresholds.is_empty() {
                let property = source.get("thresholds").unwrap();
                return Err(ConfigDiagnostic::new(
                    ConfigErrorCode::ThresholdsEmpty,
                    format!("{id} thresholds must not be empty"),
                    Some(property.value.range().into()),
                    Some(join_path(&path, "thresholds")),
                ));
            }
            let threshold_array = source
                .get("thresholds")
                .and_then(|property| property.value.as_array())
                .expect("shape validation established threshold arrays");
            for (index, pair) in thresholds.windows(2).enumerate() {
                let earlier_path = join_path(&join_path(&path, "thresholds"), &index.to_string());
                let later_path =
                    join_path(&join_path(&path, "thresholds"), &(index + 1).to_string());
                let earlier = threshold_array.elements[index]
                    .as_object()
                    .expect("shape validation established threshold objects");
                let later = threshold_array.elements[index + 1]
                    .as_object()
                    .expect("shape validation established threshold objects");
                if pair[0].limit >= pair[1].limit {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::ThresholdLimitsNotIncreasing,
                        format!("{id} threshold limits must be strictly increasing"),
                        Some(later.get("limit").unwrap().value.range().into()),
                        Some(join_path(&later_path, "limit")),
                    )
                    .with_related(
                        "previous threshold limit",
                        Some(earlier.get("limit").unwrap().value.range().into()),
                        Some(join_path(&earlier_path, "limit")),
                    ));
                }
                if pair[0].severity > pair[1].severity {
                    return Err(ConfigDiagnostic::new(
                        ConfigErrorCode::ThresholdSeverityDecreases,
                        format!("{id} threshold severity must not decrease"),
                        Some(later.get("severity").unwrap().value.range().into()),
                        Some(join_path(&later_path, "severity")),
                    )
                    .with_related(
                        "previous threshold severity",
                        Some(earlier.get("severity").unwrap().value.range().into()),
                        Some(join_path(&earlier_path, "severity")),
                    ));
                }
            }
            rule.thresholds = thresholds;
        }
    }

    if config.rules["type-enum"].enabled && config.types.is_empty() {
        let types = root
            .get("conventional")
            .and_then(|property| property.value.as_object())
            .and_then(|object| object.get("types"))
            .expect("empty types must be explicitly configured");
        let type_enum_enabled = rules_object
            .and_then(|rules| rules.get("type-enum"))
            .and_then(|property| property.value.as_object())
            .and_then(|object| object.get("enabled"));
        let related = match type_enum_enabled {
            Some(enabled) => (
                "type-enum is enabled here",
                Some(enabled.value.range().into()),
            ),
            None => ("type-enum is enabled by default", None),
        };
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::EmptyTypesWhileTypeEnumEnabled,
            "types must not be empty when type-enum is enabled",
            Some(types.value.range().into()),
            Some("/conventional/types".to_owned()),
        )
        .with_related(
            related.0,
            related.1,
            Some("/rules/type-enum/enabled".to_owned()),
        ));
    }

    if raw_scope_required(root) && config.allowed_scopes.as_ref().is_some_and(Vec::is_empty) {
        let scopes = root
            .get("conventional")
            .and_then(|property| property.value.as_object())
            .and_then(|object| object.get("allowed-scopes"))
            .expect("empty allowed-scopes must be explicitly configured");
        let scope_required = root
            .get("conventional")
            .and_then(|property| property.value.as_object())
            .and_then(|object| object.get("scope-required"))
            .expect("scope-required true must be explicitly configured");
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::ScopeRequiredWithEmptyAllowedScopes,
            "scope-required cannot be combined with empty allowed-scopes",
            Some(scopes.value.range().into()),
            Some("/conventional/allowed-scopes".to_owned()),
        )
        .with_related(
            "scope-required is enabled here",
            Some(scope_required.value.range().into()),
            Some("/conventional/scope-required".to_owned()),
        ));
    }

    if config.rules["scope-enum"].enabled && config.allowed_scopes.is_none() {
        let scope_enum_enabled = rules_object
            .and_then(|rules| rules.get("scope-enum"))
            .and_then(|property| property.value.as_object())
            .and_then(|object| object.get("enabled"))
            .expect("scope-enum cannot be enabled without an explicit override");
        return Err(ConfigDiagnostic::new(
            ConfigErrorCode::ScopeEnumWithoutAllowedScopes,
            "scope-enum requires allowed-scopes",
            None,
            Some("/conventional/allowed-scopes".to_owned()),
        )
        .with_related(
            "scope-enum is enabled here",
            Some(scope_enum_enabled.value.range().into()),
            Some("/rules/scope-enum/enabled".to_owned()),
        ));
    }

    Ok(config)
}

fn raw_scope_required(root: &Object<'_>) -> bool {
    root.get("conventional")
        .and_then(|property| property.value.as_object())
        .and_then(|object| object.get("scope-required"))
        .and_then(|property| property.value.as_boolean_lit())
        .is_some_and(|value| value.value)
}

fn expect_object<'node, 'text>(
    value: &'node Value<'text>,
    path: &str,
) -> Result<&'node Object<'text>, ConfigDiagnostic> {
    value
        .as_object()
        .ok_or_else(|| mismatch(value, path, "an object"))
}

fn expect_array<'node, 'text>(
    value: &'node Value<'text>,
    path: &str,
) -> Result<&'node jsonc_parser::ast::Array<'text>, ConfigDiagnostic> {
    value
        .as_array()
        .ok_or_else(|| mismatch(value, path, "an array"))
}

fn expect_string<'a>(value: &'a Value<'_>, path: &str) -> Result<&'a str, ConfigDiagnostic> {
    value
        .as_string_lit()
        .map(|literal| literal.value.as_ref())
        .ok_or_else(|| mismatch(value, path, "a string"))
}

fn expect_boolean(value: &Value<'_>, path: &str) -> Result<bool, ConfigDiagnostic> {
    value
        .as_boolean_lit()
        .map(|literal| literal.value)
        .ok_or_else(|| mismatch(value, path, "a boolean"))
}

fn expect_unsigned(value: &Value<'_>, path: &str, max: u64) -> Result<u64, ConfigDiagnostic> {
    let Some(number) = value.as_number_lit() else {
        return Err(mismatch(value, path, "an unsigned integer"));
    };
    let parsed = if number.value.starts_with('-') {
        None
    } else {
        number.value.parse::<u64>().ok()
    };
    match parsed {
        Some(number) if number <= max => Ok(number),
        _ => Err(ConfigDiagnostic::new(
            ConfigErrorCode::TypeMismatch,
            "expected an unsigned integer in range",
            Some(number.range.into()),
            Some(path.to_owned()),
        )),
    }
}

fn validate_string_array(value: &Value<'_>, path: &str) -> Result<(), ConfigDiagnostic> {
    let array = expect_array(value, path)?;
    for (index, value) in array.elements.iter().enumerate() {
        expect_string(value, &join_path(path, &index.to_string()))?;
    }
    Ok(())
}

fn validate_severity(value: &Value<'_>, path: &str) -> Result<(), ConfigDiagnostic> {
    let severity = expect_string(value, path)?;
    if matches!(severity, "warning" | "error") {
        Ok(())
    } else {
        Err(ConfigDiagnostic::new(
            ConfigErrorCode::InvalidEnum,
            format!("invalid severity: {severity}"),
            Some(value.range().into()),
            Some(path.to_owned()),
        ))
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
                Some(property.name.range().into()),
                Some(join_path(path, key)),
            ));
        }
    }
    Ok(())
}

fn missing_field(name: &str, path: &str) -> ConfigDiagnostic {
    ConfigDiagnostic::new(
        ConfigErrorCode::MissingField,
        format!("missing field: {name}"),
        None,
        Some(path.to_owned()),
    )
}

fn mismatch(value: &Value<'_>, path: &str, expected: &str) -> ConfigDiagnostic {
    ConfigDiagnostic::new(
        ConfigErrorCode::TypeMismatch,
        format!("expected {expected}"),
        Some(value.range().into()),
        Some(path.to_owned()),
    )
}

fn join_path(parent: &str, component: &str) -> String {
    let escaped = component.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

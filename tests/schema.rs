use commit_msg_lint::{
    config::{CONFIG_SCHEMA_URL, Config, RULES},
    schema::config_schema_json,
};
use serde_json::{Value, json};

fn schema() -> Value {
    serde_json::from_str(&config_schema_json().unwrap()).unwrap()
}

fn check(validator: &jsonschema::Validator, value: &Value, expected: bool) {
    let parsed = Config::parse(&value.to_string());
    assert_eq!(parsed.is_ok(), expected, "parser: {value}: {parsed:?}");
    assert_eq!(
        validator.is_valid(value),
        expected,
        "schema: {value}: {:?}",
        validator.iter_errors(value).collect::<Vec<_>>()
    );
}

#[test]
fn schema_is_valid_draft7_deterministic_described_and_synchronized() {
    let output = config_schema_json().unwrap();
    assert_eq!(output, config_schema_json().unwrap());
    assert_eq!(output, include_str!("../site/commit-msg-lint.schema.json"));
    let schema = schema();
    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    assert_eq!(schema["allowTrailingCommas"], true);
    assert!(jsonschema::draft7::meta::is_valid(&schema));
    let properties = schema["properties"]["rules"]["properties"]
        .as_object()
        .unwrap();
    assert_eq!(properties.len(), RULES.len());
    for id in RULES {
        assert!(
            properties[*id]["description"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "{id}"
        );
    }
    for field in [
        "$schema",
        "version",
        "conventional",
        "display-width",
        "rules",
    ] {
        assert!(
            schema["properties"][field]["description"].is_string(),
            "{field}"
        );
    }
}

#[test]
fn generated_defaults_and_metadata_are_accepted_without_fetching_urls() {
    let validator = jsonschema::draft7::new(&schema()).unwrap();
    let output = Config::default_json().unwrap();
    let defaults: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(defaults["$schema"], CONFIG_SCHEMA_URL);
    check(&validator, &defaults, true);
    assert_eq!(Config::parse(&output).unwrap(), Config::default());
    for metadata in ["", "not a URL", "https://invalid.example/schema"] {
        let value = json!({"version": 1, "$schema": metadata});
        check(&validator, &value, true);
        assert_eq!(
            Config::parse(&value.to_string()).unwrap(),
            Config::default()
        );
    }
}

#[test]
fn schema_and_parser_agree_on_structural_configuration_constraints() {
    let validator = jsonschema::draft7::new(&schema()).unwrap();
    let valid = [
        json!({"version": 1}),
        json!({"version": 1, "conventional": {}, "display-width": {}, "rules": {}}),
        json!({"version": 1, "display-width": {"ambiguous-width": 2}}),
        json!({"version": 1, "conventional": {"allowed-scopes": []}}),
        json!({"version": 1, "conventional": {"scope-required": true}}),
        json!({"version": 1, "rules": {"scope-enum": {"enabled": false}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": 0, "severity": "warning"}, {"limit": 80, "severity": "error"}]}}}),
        json!({"version": 1, "rules": {"body-max-width": {"severity": "warning"}, "footer-max-width": {"thresholds": [{"limit": 72, "severity": "error"}]}}}),
    ];
    let invalid = [
        json!({}),
        json!([]),
        json!(null),
        json!({"version": 2}),
        json!({"version": "1"}),
        json!({"version": 1, "$schema": null}),
        json!({"version": 1, "$schema": 1}),
        json!({"version": 1, "unknown": 0}),
        json!({"version": 1, "conventional": null}),
        json!({"version": 1, "conventional": {"unknown": 0}}),
        json!({"version": 1, "conventional": {"types": null}}),
        json!({"version": 1, "conventional": {"types": [1]}}),
        json!({"version": 1, "conventional": {"scope-required": null}}),
        json!({"version": 1, "conventional": {"allowed-scopes": null}}),
        json!({"version": 1, "conventional": {"allowed-scopes": [false]}}),
        json!({"version": 1, "display-width": null}),
        json!({"version": 1, "display-width": {"unknown": 0}}),
        json!({"version": 1, "display-width": {"ambiguous-width": null}}),
        json!({"version": 1, "display-width": {"ambiguous-width": 0}}),
        json!({"version": 1, "display-width": {"ambiguous-width": 3}}),
        json!({"version": 1, "conventional": {"types": []}}),
        json!({"version": 1, "rules": {"scope-enum": {"enabled": true}}}),
        json!({"version": 1, "rules": null}),
        json!({"version": 1, "rules": {"unknown": {}}}),
        json!({"version": 1, "rules": {"type-case": null}}),
        json!({"version": 1, "rules": {"type-case": {"unknown": true}}}),
        json!({"version": 1, "rules": {"type-case": {"enabled": null}}}),
        json!({"version": 1, "rules": {"type-case": {"severity": null}}}),
        json!({"version": 1, "rules": {"type-case": {"severity": "fatal"}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": null}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": []}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [null]}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"severity": "error"}]}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": 1}]}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": -1, "severity": "error"}]}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": 0.5, "severity": "error"}]}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": 0, "severity": null}]}}}),
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": 1, "severity": "error", "extra": 0}]}}}),
        json!({"version": 1, "rules": {"body-max-width": {"enabled": false, "severity": "error", "thresholds": [{"limit": 1, "severity": "error"}]}}}),
    ];
    for value in valid {
        check(&validator, &value, true);
    }
    for value in invalid {
        check(&validator, &value, false);
    }
}

#[test]
fn every_rule_has_exactly_its_supported_properties() {
    let validator = jsonschema::draft7::new(&schema()).unwrap();
    for &id in RULES {
        let header_width = id == "header-max-width";
        let line_width = ["body-max-width", "footer-max-width"].contains(&id);
        for (rule, expected) in [
            (json!({}), true),
            (json!({"enabled": false}), true),
            (json!({"severity": "warning"}), !header_width),
            (
                json!({"thresholds": [{"limit": 10, "severity": "warning"}]}),
                header_width || line_width,
            ),
            (
                json!({"severity": "warning", "thresholds": [{"limit": 10, "severity": "warning"}]}),
                false,
            ),
        ] {
            let value = json!({"version": 1, "rules": {id: rule}});
            check(&validator, &value, expected);
        }
    }
}

#[test]
fn cross_field_combinations_match_parser_including_default_and_overridden_rule_states() {
    let validator = jsonschema::draft7::new(&schema()).unwrap();
    for types in [None, Some(json!([])), Some(json!(["feat"]))] {
        for required in [None, Some(false), Some(true)] {
            for scopes in [None, Some(json!([])), Some(json!(["cli"]))] {
                for type_enabled in [None, Some(false), Some(true)] {
                    for scope_enabled in [None, Some(false), Some(true)] {
                        for required_enabled in [None, Some(false), Some(true)] {
                            let mut value = json!({"version": 1, "conventional": {}, "rules": {}});
                            if let Some(v) = &types {
                                value["conventional"]["types"] = v.clone();
                            }
                            if let Some(v) = required {
                                value["conventional"]["scope-required"] = json!(v);
                            }
                            if let Some(v) = &scopes {
                                value["conventional"]["allowed-scopes"] = v.clone();
                            }
                            for (id, enabled) in [
                                ("type-enum", type_enabled),
                                ("scope-enum", scope_enabled),
                                ("scope-required", required_enabled),
                            ] {
                                if let Some(enabled) = enabled {
                                    value["rules"][id] = json!({"enabled": enabled});
                                }
                            }
                            let expected = !(types == Some(json!([]))
                                && type_enabled != Some(false)
                                || required == Some(true) && scopes == Some(json!([]))
                                || scope_enabled == Some(true) && scopes.is_none());
                            check(&validator, &value, expected);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn parser_checks_ordering_and_jsonc_syntax_beyond_schema_validation() {
    let validator = jsonschema::draft7::new(&schema()).unwrap();
    for value in [
        json!({"version": 1, "rules": {"header-max-width": {"thresholds": [{"limit": 10, "severity": "warning"}, {"limit": 10, "severity": "error"}]}}}),
        json!({"version": 1, "rules": {"body-max-width": {"thresholds": [{"limit": 10, "severity": "error"}, {"limit": 20, "severity": "warning"}]}}}),
    ] {
        assert!(validator.is_valid(&value));
        assert!(Config::parse(&value.to_string()).is_err());
    }
    assert!(Config::parse("{\"version\":1,\"version\":1}").is_err());
    assert!(Config::parse("{\"version\":1,\"$schema\":\"a\",\"$schema\":\"b\"}").is_err());
    assert!(Config::parse("{ // comment\n \"version\":1, }").is_ok());
}

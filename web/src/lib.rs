use commit_msg_lint::{config::Config, lint::lint};
use serde_json::json;
use wasm_bindgen::prelude::*;

/// Returns the built-in defaults as plain, pretty-printed JSON.
#[wasm_bindgen(js_name = defaultConfig)]
pub fn default_config() -> Result<String, String> {
    Config::default_json().map_err(|e| format!("could not serialize default configuration: {e}"))
}

/// Lints `message` with JSONC `config` (empty means defaults) and returns a JSON object containing
/// `lintDiagnostics` and `configDiagnostics` arrays.
#[wasm_bindgen(js_name = lint)]
pub fn lint_message(message: &str, config: &str) -> Result<String, String> {
    let config = if config.trim().is_empty() {
        Config::default()
    } else {
        match Config::parse(config) {
            Ok(config) => config,
            Err(diagnostic) => {
                return Ok(json!({
                    "lintDiagnostics": [],
                    "configDiagnostics": [diagnostic],
                })
                .to_string());
            }
        }
    };
    let diagnostics: Vec<_> = lint(message, &config)
        .into_iter()
        .map(|d| {
            json!({
                "line": d.line,
                "rule": d.rule,
                "severity": d.severity.to_string(),
                "message": d.message,
            })
        })
        .collect();
    Ok(json!({
        "lintDiagnostics": diagnostics,
        "configDiagnostics": [],
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(message: &str, config: &str) -> serde_json::Value {
        serde_json::from_str(&lint_message(message, config).unwrap()).unwrap()
    }

    #[test]
    fn default_configuration_exposes_ignore_settings() {
        let defaults: serde_json::Value = serde_json::from_str(&default_config().unwrap()).unwrap();
        assert_eq!(
            defaults["ignore"],
            json!({"merge": true, "revert": true, "autosquash": true, "header-patterns": []})
        );
    }

    #[test]
    fn generated_headers_ignore_the_whole_message_by_default() {
        let body = format!("\n\n{}\ninvalid\u{0001}", "x".repeat(100));
        for header in [
            "fixup! ",
            "amend! invalid",
            "squash! fixup! x",
            "Merge branch x",
            "Revert x",
        ] {
            let result = response(&format!("{header}{body}"), "");
            assert_eq!(
                result,
                json!({"lintDiagnostics": [], "configDiagnostics": []}),
                "{header}"
            );
        }
    }

    #[test]
    fn built_in_groups_can_be_disabled_independently() {
        for (group, header) in [
            ("merge", "Merge branch x"),
            ("revert", "Revert x"),
            ("autosquash", "fixup! x"),
        ] {
            let config = format!(r#"{{"version":1,"ignore":{{"{group}":false}}}}"#);
            let diagnostics = response(header, &config)["lintDiagnostics"].clone();
            assert!(
                diagnostics
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["rule"] == "header-format"),
                "{group}: {diagnostics}"
            );
            let other_header = if group == "merge" {
                "Revert x"
            } else {
                "Merge branch x"
            };
            assert_eq!(
                response(other_header, &config),
                json!({"lintDiagnostics": [], "configDiagnostics": []})
            );
        }
    }

    #[test]
    fn custom_header_patterns_remain_active_when_built_ins_are_disabled() {
        let config = r#"{
            "version": 1,
            "ignore": {
                "merge": false, "revert": false, "autosquash": false,
                "header-patterns": ["^custom: ", "日本語"]
            }
        }"#;
        for message in ["custom: invalid\n\n\u{0001}", "bad 日本語 message"] {
            assert_eq!(
                response(message, config),
                json!({"lintDiagnostics": [], "configDiagnostics": []}),
                "{message}"
            );
        }
        for message in ["bad header", "bad header\n\n日本語"] {
            let diagnostics = response(message, config)["lintDiagnostics"].clone();
            assert!(
                diagnostics
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["rule"] == "header-format")
            );
        }
    }

    #[test]
    fn web_checks_the_unfiltered_first_header_and_conventional_reverts() {
        for message in [
            "# comment\nMerge branch feature",
            "bad header\n\nMerge branch feature",
            "revert: subject.",
        ] {
            let diagnostics = response(message, "")["lintDiagnostics"].clone();
            assert!(!diagnostics.as_array().unwrap().is_empty(), "{message}");
        }
    }

    #[test]
    fn invalid_ignore_configuration_is_reported_before_ignoring() {
        for config in [
            r#"{"version":1,"ignore":{"merge":null}}"#,
            r#"{"version":1,"ignore":{"header-patterns":[""]}}"#,
            r#"{"version":1,"ignore":{"header-patterns":["["]}}"#,
        ] {
            let result = response("fixup! invalid", config);
            assert!(result["lintDiagnostics"].as_array().unwrap().is_empty());
            assert_eq!(result["configDiagnostics"].as_array().unwrap().len(), 1);
            assert!(result["configDiagnostics"][0]["code"].is_string());
        }
    }

    #[test]
    fn reports_diagnostics_as_json() {
        let result = response("unknown: x", "");
        assert!(
            result["lintDiagnostics"]
                .to_string()
                .contains("\"rule\":\"type-enum\"")
        );
        assert!(result["configDiagnostics"].as_array().unwrap().is_empty());
        assert_eq!(result.as_object().unwrap().len(), 2);
    }

    #[test]
    fn lint_severity_stays_separate_from_configuration_errors() {
        let warning = response(
            "bad header",
            r#"{"version":1,"rules":{"header-format":{"severity":"warning"}}}"#,
        );
        assert!(
            warning["lintDiagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| { d["rule"] == "header-format" && d["severity"] == "warning" })
        );
        assert!(warning["configDiagnostics"].as_array().unwrap().is_empty());

        let invalid = response("bad header", r#"{"version":1,"unknown":true}"#);
        assert!(invalid["lintDiagnostics"].as_array().unwrap().is_empty());
        assert_eq!(
            invalid["configDiagnostics"][0]["code"],
            "config.unknown-field"
        );
        assert!(invalid["configDiagnostics"][0].get("severity").is_none());
    }

    #[test]
    fn default_configuration_json_produces_the_same_diagnostics_as_empty_configuration() {
        let defaults = default_config().unwrap();
        for message in [
            "feat: example",
            "unknown: x",
            "feat: this is a long subject that exceeds the recommended header width",
        ] {
            assert_eq!(lint_message(message, &defaults), lint_message(message, ""));
        }
    }

    #[test]
    fn accepts_jsonc_configuration_with_comments_and_trailing_commas() {
        let config = r#"{
            // Override the allowed types.
            "version": 1,
            "conventional": { "types": ["fix",], },
        }"#;
        let result = response("feat: x", config);
        assert!(
            result["lintDiagnostics"]
                .to_string()
                .contains("\"rule\":\"type-enum\"")
        );
    }

    #[test]
    fn ignores_schema_metadata_but_requires_a_string() {
        assert_eq!(
            lint_message(
                "feat: x",
                r#"{"version":1,"$schema":"https://invalid.example/schema"}"#
            ),
            lint_message("feat: x", r#"{"version":1}"#)
        );
        let invalid = response("feat: x", r#"{"version":1,"$schema":null}"#);
        assert_eq!(invalid["configDiagnostics"].as_array().unwrap().len(), 1);
        assert!(invalid["lintDiagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_configuration() {
        for config in [
            r#"{"version":2}"#,
            r#"{"version":1,"conventional":{"types":null}}"#,
            r#"{"version":1,"rules":{"type-case":{},"type-case":{}}}"#,
        ] {
            let result = response("feat: x", config);
            assert_eq!(
                result["configDiagnostics"].as_array().unwrap().len(),
                1,
                "{config}"
            );
            assert!(
                result["lintDiagnostics"].as_array().unwrap().is_empty(),
                "{config}"
            );
        }
    }
}

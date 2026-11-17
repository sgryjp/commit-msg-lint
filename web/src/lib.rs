use commit_msg_lint::{config::Config, lint::lint};
use serde_json::json;
use wasm_bindgen::prelude::*;

/// Returns the built-in defaults as plain, pretty-printed JSON.
#[wasm_bindgen(js_name = defaultConfig)]
pub fn default_config() -> Result<String, String> {
    Config::default_json().map_err(|e| format!("could not serialize default configuration: {e}"))
}

/// Lints `message` with JSONC `config` (empty means defaults) and returns a JSON array of
/// `{line, rule, severity, message}` objects.
#[wasm_bindgen(js_name = lint)]
pub fn lint_message(message: &str, config: &str) -> Result<String, String> {
    let config = if config.trim().is_empty() {
        Config::default()
    } else {
        Config::parse(config).map_err(|e| format!("invalid configuration: {e}"))?
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
    Ok(serde_json::Value::Array(diagnostics).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let result = lint_message(&format!("{header}{body}"), "").unwrap();
            assert_eq!(result, "[]", "{header}");
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
            let diagnostics: serde_json::Value =
                serde_json::from_str(&lint_message(header, &config).unwrap()).unwrap();
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
            assert_eq!(lint_message(other_header, &config).unwrap(), "[]");
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
            assert_eq!(lint_message(message, config).unwrap(), "[]", "{message}");
        }
        for message in ["bad header", "bad header\n\n日本語"] {
            let diagnostics: serde_json::Value =
                serde_json::from_str(&lint_message(message, config).unwrap()).unwrap();
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
            let diagnostics: serde_json::Value =
                serde_json::from_str(&lint_message(message, "").unwrap()).unwrap();
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
            let error = lint_message("fixup! invalid", config).unwrap_err();
            assert!(
                error.starts_with("invalid configuration:"),
                "{config}: {error}"
            );
        }
    }

    #[test]
    fn reports_diagnostics_as_json() {
        let json = lint_message("unknown: x", "").unwrap();
        assert!(json.contains("\"rule\":\"type-enum\""));
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
        let result = lint_message("feat: x", config).unwrap();
        assert!(result.contains("\"rule\":\"type-enum\""));
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
        assert!(lint_message("feat: x", r#"{"version":1,"$schema":null}"#).is_err());
    }

    #[test]
    fn rejects_invalid_configuration() {
        for config in [
            r#"{"version":2}"#,
            r#"{"version":1,"conventional":{"types":null}}"#,
            r#"{"version":1,"rules":{"type-case":{},"type-case":{}}}"#,
        ] {
            assert!(
                lint_message("feat: x", config).is_err(),
                "accepted {config}"
            );
        }
    }
}

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

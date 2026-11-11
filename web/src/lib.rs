use commit_msg_lint::{config::Config, lint::lint};
use serde_json::json;
use wasm_bindgen::prelude::*;

/// Lints `message` with `config_toml` (empty means defaults) and returns a JSON array of
/// `{line, rule, severity, message}` objects.
#[wasm_bindgen(js_name = lint)]
pub fn lint_message(message: &str, config_toml: &str) -> Result<String, String> {
    let config = if config_toml.trim().is_empty() {
        Config::default()
    } else {
        Config::parse(config_toml).map_err(|e| format!("invalid configuration: {e}"))?
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
    fn rejects_invalid_configuration() {
        assert!(lint_message("feat: x", "version = 2").is_err());
    }
}

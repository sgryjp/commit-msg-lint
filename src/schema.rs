//! Generation of the configuration schema for the running version.

use serde_json::{Map, json};

use crate::config::{Config, RULE_METADATA, RuleKind};

/// Returns a deterministic, pretty-printed Draft-07 configuration schema.
pub fn config_schema_json() -> Result<String, serde_json::Error> {
    let mut rules = Map::new();
    for metadata in RULE_METADATA {
        let mut properties = Map::new();
        let mut enabled = json!({
            "type": "boolean",
            "description": "A disabled Rule produces no diagnostics."
        });
        if metadata.kind != RuleKind::Scope {
            enabled["default"] = json!(true);
        }
        properties.insert("enabled".into(), enabled);
        if metadata.kind.supports_severity() {
            properties.insert(
                "severity".into(),
                json!({
                    "type": "string", "enum": ["warning", "error"], "default": "error",
                    "description": "Diagnostic severity. For body and footer width, changes the severity of default thresholds."
                }),
            );
        }
        if metadata.kind.supports_thresholds() {
            properties.insert(
                "thresholds".into(),
                json!({
                    "type": "array", "minItems": 1,
                    "items": {"$ref": "#/definitions/threshold"},
                    "description": "Replaces default thresholds. Limits must increase strictly; severity must stay the same or increase from warning to error."
                }),
            );
        }
        let mut rule = json!({
            "type": "object", "description": metadata.description,
            "properties": properties, "additionalProperties": false
        });
        if metadata.kind.supports_severity() && metadata.kind.supports_thresholds() {
            rule["not"] = json!({"required": ["severity", "thresholds"]});
        }
        rules.insert(metadata.id.into(), rule);
    }

    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "commit-msg-lint configuration",
        "description": "JSONC configuration overrides built-in defaults. The parser also validates JSONC syntax, duplicate keys, and threshold ordering.",
        "allowTrailingCommas": true,
        "type": "object",
        "required": ["version"],
        "additionalProperties": false,
        "properties": {
            "$schema": {
                "type": "string",
                "description": "Optional editor schema association; ignored by the parser."
            },
            "version": {
                "type": "integer", "const": 1,
                "description": "Required configuration format version. The pre-1.0 schema may change."
            },
            "conventional": {
                "type": "object", "additionalProperties": false,
                "description": "Conventional Commit settings.",
                "properties": {
                    "types": {
                        "type": "array", "items": {"type": "string"},
                        "default": Config::default().types,
                        "description": "Replaces the allowed Type list; matching is ASCII case-insensitive. Must be nonempty when type-enum is enabled."
                    },
                    "scope-required": {
                        "type": "boolean", "default": false,
                        "description": "Enables the scope-required Rule; Rule-level enabled overrides it. Cannot be true with empty allowed-scopes."
                    },
                    "allowed-scopes": {
                        "type": "array", "items": {"type": "string"},
                        "description": "Allowed Scopes, matched exactly and case-sensitively. Setting this enables scope-enum unless overridden; an empty array permits no Scope."
                    }
                }
            },
            "display-width": {
                "type": "object", "additionalProperties": false,
                "description": "Display width calculation settings.",
                "properties": {
                    "ambiguous-width": {
                        "type": "integer", "enum": [1, 2], "default": 1,
                        "description": "Use normal (1) or CJK (2) width for characters with ambiguous display width."
                    }
                }
            },
            "ignore": {
                "type": "object", "additionalProperties": false,
                "description": "Ignore entire messages whose Header matches a built-in group or a custom regex-lite pattern. Regex syntax is validated by the parser.",
                "properties": {
                    "merge": {"type": "boolean", "default": true, "description": "Ignore recognized merge Headers."},
                    "revert": {"type": "boolean", "default": true, "description": "Ignore recognized revert and reapply Headers."},
                    "autosquash": {"type": "boolean", "default": true, "description": "Ignore recognized fixup, amend, and squash Headers."},
                    "header-patterns": {
                        "type": "array", "default": [],
                        "items": {"type": "string", "minLength": 1},
                        "description": "Additional nonempty regex-lite patterns searched within the first Header."
                    }
                }
            },
            "rules": {
                "type": "object", "additionalProperties": false, "properties": rules,
                "description": "Rule overrides are validated even when the Rule is disabled."
            }
        },
        "definitions": {
            "threshold": {
                "type": "object", "additionalProperties": false,
                "required": ["limit", "severity"],
                "properties": {
                    "limit": {"type": "integer", "minimum": 0, "description": "A line violates this threshold when its width is greater than this limit."},
                    "severity": {"type": "string", "enum": ["warning", "error"], "description": "Severity reported when this threshold is exceeded."}
                }
            }
        },
        "allOf": [
            {
                "if": {
                    "required": ["conventional"],
                    "properties": {"conventional": {
                        "required": ["types"], "properties": {"types": {"maxItems": 0}}
                    }}
                },
                "then": {
                    "required": ["rules"], "properties": {"rules": {
                        "required": ["type-enum"], "properties": {"type-enum": {
                            "required": ["enabled"], "properties": {"enabled": {"const": false}}
                        }}
                    }}
                }
            },
            {
                "if": {
                    "required": ["conventional"], "properties": {"conventional": {
                        "required": ["scope-required"], "properties": {"scope-required": {"const": true}}
                    }}
                },
                "then": {"properties": {"conventional": {
                    "properties": {"allowed-scopes": {"minItems": 1}}
                }}}
            },
            {
                "if": {
                    "required": ["rules"], "properties": {"rules": {
                        "required": ["scope-enum"], "properties": {"scope-enum": {
                            "required": ["enabled"], "properties": {"enabled": {"const": true}}
                        }}
                    }}
                },
                "then": {
                    "required": ["conventional"], "properties": {"conventional": {"required": ["allowed-scopes"]}}
                }
            }
        ]
    });
    serde_json::to_string_pretty(&schema)
}

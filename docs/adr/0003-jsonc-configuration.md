# Adopt JSONC for Configuration Files

[日本語](0003-jsonc-configuration.ja.md)

**Status: implemented.**

Use JSONC, allowing comments and trailing commas, as the configuration file format.

Although TOML is easy for humans to write, schema-based autocompletion and validation in text
editors require additional mechanisms. JSONC, by contrast, has ecosystem support for autocompletion
and validation using JSON Schema while also being easy for humans to write. This is why it was
chosen. However, JSONC syntax and handling of trailing commas vary between tools, so the CLI and web
demo use the same configuration parsing to align accepted syntax and validation results.

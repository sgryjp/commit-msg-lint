# Use JSONC for configuration files

**Status: Implemented.**

Use JSONC, which allows comments and trailing commas, as the configuration file format.

Although TOML is easy for people to write, providing editor autocomplete and validation based on
a schema requires additional mechanisms. In contrast, the ecosystem supports autocomplete and
validation for JSONC using JSON Schema, while JSONC also remains easy for people to write. This is
why JSONC was chosen.

However, tools differ in the JSONC syntax they accept and how they handle trailing commas. The CLI
and web demo therefore use the same configuration parser so that they accept the same syntax and
produce consistent validation results.

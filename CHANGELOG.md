# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Configurable ignores for recognized Git- and hosting-service-generated merge, revert, and
  autosquash messages, with custom Header patterns; matched messages skip all lint Rules.
- GitHub Releases are created automatically when a version tag is pushed.

## [0.0.1] - 2026-11-17

### Added

- A CLI and Web demo for checking commit messages against configurable Conventional Commit Rules.
- Reading messages from standard input or a file, with configurable diagnostics and strict mode.
- JSONC configuration with built-in defaults, default-configuration export, and a published JSON Schema.
- CLI filtering of Git comment lines and scissors sections, with environment variables to control each behavior.
- Agent-oriented usage guidance through `--help-for-agents`.
- Display-width-aware checks, including configurable handling of East Asian ambiguous-width characters.

### Changed

- The CLI, library, and Web demo share the same linting and configuration behavior;
  the CLI applies Git message filtering before linting by default.

[Unreleased]: https://github.com/sgryjp/commit-msg-lint/compare/0.0.1...HEAD
[0.0.1]: https://github.com/sgryjp/commit-msg-lint/releases/tag/0.0.1

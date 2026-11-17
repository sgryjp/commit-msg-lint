use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "commit-msg-lint-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn isolated_command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.0)
            .env("USERPROFILE", &self.0)
            .env("XDG_CONFIG_HOME", self.0.join("no-xdg-config"))
            .env("GIT_CONFIG_GLOBAL", self.0.join("no-global-config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(&self.0);
        let parent = self.0.parent().unwrap();
        let separator = if cfg!(windows) { b';' } else { b':' };
        if !parent.as_os_str().as_encoded_bytes().contains(&separator) {
            command.env("GIT_CEILING_DIRECTORIES", parent);
        }
        #[cfg(windows)]
        for name in ["SystemRoot", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command
    }

    fn git(&self, args: &[&str]) {
        let output = self.isolated_command("git").args(args).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {}", stderr(&output));
    }

    fn command(&self, args: &[&str], input: &[u8]) -> Output {
        self.command_with_env(args, input, &[])
    }

    fn command_with_env(&self, args: &[&str], input: &[u8], env: &[(&str, &str)]) -> Output {
        let mut command = self.isolated_command(env!("CARGO_BIN_EXE_commit-msg-lint"));
        command.envs(env.iter().copied());
        let mut child = command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // The process may exit before reading stdin (e.g. argument errors).
        let _ = child.stdin.take().unwrap().write_all(input);
        child.wait_with_output().unwrap()
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}

#[test]
fn help_explains_how_each_option_affects_linting() {
    let dir = TestDir::new();
    let output = dir.command(&["--help"], b"");
    assert_eq!(output.status.code(), Some(0));
    let help = std::str::from_utf8(&output.stdout).unwrap();
    assert!(help.contains("Read the message from a file instead of standard input."));
    assert!(help.contains("Use the specified configuration file instead of discovering one."));
    assert!(
        help.contains(
            "Treat warnings as failures for the exit code without changing their severity."
        )
    );
    assert!(help.contains("--help-for-agents"));
    assert!(help.contains("Print agent-oriented usage guidance and exit."));
    assert!(help.contains("--print-default-config"));
    assert!(help.contains("Print the built-in default configuration as JSON and exit."));
    assert!(help.contains("--print-config-schema"));
    assert!(help.contains("Print the configuration JSON Schema and exit."));
}

#[test]
fn print_default_config_ignores_existing_config_git_and_filter_environment() {
    let dir = TestDir::new();
    fs::write(dir.0.join("commit-msg-lint.jsonc"), "not valid JSONC").unwrap();
    let output = dir.command_with_env(
        &["--print-default-config"],
        b"input that must not be linted",
        &[
            ("PATH", ""),
            ("GIT_DIR", "missing"),
            ("GIT_CONFIG_COUNT", "invalid"),
            (STRIP, "invalid"),
            (CUT, ""),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let config = std::str::from_utf8(&output.stdout).unwrap();
    let _: serde_json::Value = serde_json::from_str(config).expect("defaults must be plain JSON");
    assert_eq!(
        config,
        commit_msg_lint::config::Config::default_json().unwrap()
    );
    assert_eq!(
        commit_msg_lint::config::Config::parse(config).unwrap(),
        commit_msg_lint::config::Config::default()
    );
}

fn assert_does_not_wait_for_standard_input_eof(option: &str) {
    let dir = TestDir::new();
    let mut child = dir
        .isolated_command(env!("CARGO_BIN_EXE_commit-msg-lint"))
        .arg(option)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            drop(stdin);
            let _ = child.wait();
            panic!("{option} waited for standard input EOF");
        }
        thread::sleep(Duration::from_millis(10));
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}

#[test]
fn print_default_config_does_not_wait_for_standard_input_eof() {
    assert_does_not_wait_for_standard_input_eof("--print-default-config");
}

#[test]
fn print_default_config_cannot_be_combined_with_other_arguments() {
    let dir = TestDir::new();
    for args in [
        &["--print-default-config", "--strict"][..],
        &["--strict", "--print-default-config"],
        &["--print-default-config", "--edit", "message"],
        &["--print-default-config", "--config", "config.jsonc"],
        &["--print-default-config", "--help"],
        &["--print-default-config", "-h"],
        &["--print-default-config", "--version"],
        &["--print-default-config", "-V"],
        &["--print-default-config", "--print-default-config"],
    ] {
        let output = dir.command(args, b"ignored");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(stderr(&output).contains("--print-default-config cannot be combined"));
    }
}

#[test]
fn print_config_schema_ignores_config_git_and_filter_environment() {
    let dir = TestDir::new();
    fs::write(dir.0.join("commit-msg-lint.jsonc"), "invalid").unwrap();
    let output = dir.command_with_env(
        &["--print-config-schema"],
        b"input that must not be linted",
        &[
            ("PATH", ""),
            ("GIT_DIR", "missing"),
            ("GIT_CONFIG_COUNT", "invalid"),
            (STRIP, "invalid"),
            (CUT, ""),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        commit_msg_lint::schema::config_schema_json()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        output.stdout,
        dir.command(&["--print-config-schema"], b"").stdout
    );
}

#[test]
fn print_config_schema_does_not_wait_for_standard_input_eof() {
    assert_does_not_wait_for_standard_input_eof("--print-config-schema");
}

#[test]
fn print_config_schema_cannot_be_combined_with_other_arguments() {
    let dir = TestDir::new();
    for args in [
        &["--print-config-schema", "--strict"][..],
        &["--strict", "--print-config-schema"],
        &["--print-config-schema", "--edit", "message"],
        &["--print-config-schema", "--config", "config.jsonc"],
        &["--print-config-schema", "--help"],
        &["--print-config-schema", "-h"],
        &["--print-config-schema", "--version"],
        &["--print-config-schema", "-V"],
        &["--print-config-schema", "--print-config-schema"],
        &["--print-config-schema", "positional"],
        &["--print-config-schema", "--print-default-config"],
        &["--help-for-agents", "--print-config-schema"],
    ] {
        let output = dir.command(args, b"ignored");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            stderr(&output).contains("cannot be combined"),
            "{args:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn agent_help_prints_bundled_markdown_without_reading_stdin_git_config_or_policy() {
    let dir = TestDir::new();
    fs::create_dir(dir.0.join(".git")).unwrap();
    fs::write(dir.0.join(".git/config"), "not valid Git configuration").unwrap();
    fs::write(dir.0.join("commit-msg-lint.jsonc"), "not valid JSONC").unwrap();

    let output = dir.command_with_env(
        &["--help-for-agents"],
        b"input that must not be linted",
        &[
            ("PATH", ""),
            ("GIT_DIR", "missing"),
            ("GIT_CONFIG_COUNT", "invalid"),
            ("GIT_CONFIG_GLOBAL", "missing-config"),
            (STRIP, "invalid"),
            (CUT, ""),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, include_bytes!("../docs/agent-usage.md"));
    assert!(output.stderr.is_empty());
}

#[test]
fn agent_help_does_not_wait_for_standard_input_eof() {
    assert_does_not_wait_for_standard_input_eof("--help-for-agents");
}

#[test]
fn agent_help_cannot_be_combined_with_other_arguments() {
    let dir = TestDir::new();
    for args in [
        &["--help-for-agents", "--strict"][..],
        &["--strict", "--help-for-agents"],
        &["--help-for-agents", "--edit", "message"],
        &["--help-for-agents", "--config", "config.jsonc"],
        &["--help-for-agents", "--print-default-config"],
        &["--print-default-config", "--help-for-agents"],
        &["--help-for-agents", "--help"],
        &["--help-for-agents", "-h"],
        &["--help-for-agents", "--version"],
        &["--help-for-agents", "-V"],
    ] {
        let output = dir.command(args, b"ignored");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(stderr(&output).contains("--help-for-agents cannot be combined"));
    }
}

#[test]
fn agent_usage_documents_completed_message_and_existing_hook_examples() {
    let docs = include_str!("../docs/agent-usage.md");
    assert!(docs.contains("COMMIT_MSG_LINT_STRIP_COMMENTS=false"));
    assert!(docs.contains("COMMIT_MSG_LINT_CUT_SCISSORS=false"));
    assert!(docs.contains("git -c commit.verbose=false commit --cleanup=verbatim -F message.txt"));
    assert!(
        docs.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("unless the hook explicitly removes or replaces them")
    );
    assert!(docs.contains("warnings can still be printed to\nstandard error with exit code `0`"));
}

#[test]
fn standard_input_and_strict_exit_codes_match_severity() {
    let dir = TestDir::new();
    let output = dir.command(&[], b"feat: add parser\n");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());

    for (length, strict, code, severity) in [
        (51, false, 0, "warning"),
        (51, true, 1, "warning"),
        (72, false, 0, "warning"),
        (73, false, 1, "error"),
    ] {
        let input = format!("feat: {}", "x".repeat(length - 6));
        let args: &[&str] = if strict { &["--strict"] } else { &[] };
        let output = dir.command(args, input.as_bytes());
        assert_eq!(output.status.code(), Some(code));
        assert!(stderr(&output).contains(&format!("<stdin>:1: {severity}[header-max-width]")));
        assert_eq!(stderr(&output).lines().count(), 1);
    }
}

#[test]
fn long_options_accept_equals_syntax() {
    let dir = TestDir::new();
    fs::write(dir.0.join("message"), "feat: change\n").unwrap();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();

    let output = dir.command(&["--config=rules.jsonc", "--edit=message"], b"ignored");

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
}

#[test]
fn explicit_configuration_and_edit_path_are_used() {
    let dir = TestDir::new();
    fs::write(dir.0.join("message"), "feat(cli): change\n").unwrap();
    fs::write(
        dir.0.join("rules.jsonc"),
        r#"{ "version": 1, "conventional": { "allowed-scopes": [] } }"#,
    )
    .unwrap();
    let output = dir.command(
        &["--config", "rules.jsonc", "--edit", "message"],
        b"ignored",
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("message:1: error[scope-enum]"));
    fs::write(
        dir.0.join("rules.jsonc"),
        r#"{ "version": 1, "conventional": { "allowed-scopes": [] }, "rules": { "scope-enum": { "enabled": false } } }"#,
    )
    .unwrap();
    assert_eq!(
        dir.command(&["--config", "rules.jsonc", "--edit", "message"], b"")
            .status
            .code(),
        Some(0)
    );
}

#[test]
fn repository_root_configuration_is_discovered_from_subdirectories() {
    let dir = TestDir::new();
    dir.git(&["init", "-q"]);
    fs::write(
        dir.0.join("commit-msg-lint.jsonc"),
        r#"{
            // The root policy applies even in subdirectories.
            "version": 1,
            "conventional": { /* JSONC comments */ "types": ["fix",], },
        }"#,
    )
    .unwrap();
    fs::create_dir(dir.0.join("child")).unwrap();
    fs::write(dir.0.join("child/commit-msg-lint.jsonc"), "not valid JSONC").unwrap();
    let mut child = dir
        .isolated_command(env!("CARGO_BIN_EXE_commit-msg-lint"))
        .current_dir(dir.0.join("child"))
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"feat: change")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("error[type-enum]"));
}

#[test]
fn explicit_jsonc_is_used_regardless_of_extension_and_overrides_discovery() {
    let dir = TestDir::new();
    dir.git(&["init", "-q"]);
    fs::write(
        dir.0.join("commit-msg-lint.jsonc"),
        "invalid discovered config",
    )
    .unwrap();
    for name in ["policy.toml", "policy", "policy.jsonc"] {
        fs::write(
            dir.0.join(name),
            r#"{
                // Explicit paths contain JSONC, not TOML, regardless of extension.
                "version": 1,
                "conventional": { "types": ["fix",], }, /* comment */
            }"#,
        )
        .unwrap();
        let output = dir.command(&["--config", name], b"feat: change\n");
        assert_eq!(output.status.code(), Some(1), "{name}: {}", stderr(&output));
        assert!(stderr(&output).contains("error[type-enum]"), "{name}");
        let output = dir.command(&["--config", name], b"fix: change\n");
        assert_eq!(output.status.code(), Some(0), "{name}: {}", stderr(&output));
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn absent_auto_config_uses_defaults_and_legacy_toml_is_ignored() {
    let dir = TestDir::new();
    dir.git(&["init", "-q"]);
    for legacy in [
        None,
        Some("version = 1\n[conventional]\ntypes = ['fix']\n"),
        Some("invalid TOML"),
    ] {
        if let Some(content) = legacy {
            fs::write(dir.0.join("commit-msg-lint.toml"), content).unwrap();
        }
        let output = dir.command(&[], b"feat: change\n");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{legacy:?}: {}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
}

#[test]
fn git_ceiling_prevents_discovery_of_an_ancestor_jsonc_config() {
    let dir = TestDir::new();
    let separator = if cfg!(windows) { b';' } else { b':' };
    if dir.0.as_os_str().as_encoded_bytes().contains(&separator) {
        return;
    }
    dir.git(&["init", "-q"]);
    fs::write(
        dir.0.join("commit-msg-lint.jsonc"),
        r#"{ "version": 1, "conventional": { "types": ["fix"] } }"#,
    )
    .unwrap();
    fs::create_dir(dir.0.join("child")).unwrap();

    for (ceiling, expected) in [(dir.0.parent().unwrap(), 1), (dir.0.as_path(), 0)] {
        let mut child = dir
            .isolated_command(env!("CARGO_BIN_EXE_commit-msg-lint"))
            .current_dir(dir.0.join("child"))
            .env("GIT_CEILING_DIRECTORIES", ceiling)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"feat: change\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(expected), "{}", stderr(&output));
        if expected == 1 {
            assert!(stderr(&output).contains("error[type-enum]"));
        } else {
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
fn invalid_jsonc_configuration_is_a_cli_error() {
    let dir = TestDir::new();
    for (description, content) in [
        (
            "explicit null",
            r#"{ "version": 1, "conventional": { "allowed-scopes": null } }"#,
        ),
        ("duplicate field", r#"{ "version": 1, "version": 1 }"#),
        (
            "duplicate rule",
            r#"{ "version": 1, "rules": { "type-enum": {}, "type-enum": {} } }"#,
        ),
        (
            "escaped duplicate rule",
            r#"{ "version": 1, "rules": { "type-enum": {}, "type-\u0065num": {} } }"#,
        ),
        ("unquoted key", r#"{ version: 1 }"#),
        (
            "single-quoted string",
            r#"{ "version": 1, "conventional": { "types": ['fix'] } }"#,
        ),
        ("unknown field", r#"{ "version": 1, "unknown": 1 }"#),
        ("invalid syntax", r#"{ "version": 1, "rules": [ }"#),
        ("TOML is not JSONC", "version = 1\n"),
    ] {
        fs::write(dir.0.join("invalid.jsonc"), content).unwrap();
        let output = dir.command(&["--config", "invalid.jsonc"], b"feat: change\n");
        assert_eq!(
            output.status.code(),
            Some(2),
            "{description}: {}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty(), "{description}");
        assert!(!output.stderr.is_empty(), "{description}");
    }
}

#[test]
fn malformed_inputs_and_configuration_are_errors() {
    let dir = TestDir::new();
    fs::write(dir.0.join("bad.jsonc"), r#"{ "version": 1, "unknown": 1 }"#).unwrap();
    fs::write(dir.0.join("bad-utf8"), [0xff]).unwrap();
    for (args, input) in [
        (vec!["--unknown"], &b"feat: change"[..]),
        (vec!["--edit"], &b""[..]),
        (vec!["--edit", "bad-utf8"], &b""[..]),
        (vec!["--edit", "missing"], &b""[..]),
        (vec!["--config", "bad.jsonc"], &b"feat: change"[..]),
        (vec!["--config", "missing"], &b"feat: change"[..]),
        (vec![], &[0xff][..]),
    ] {
        let output = dir.command(&args, input);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn diagnostics_are_sorted_and_newline_encodings_are_equivalent() {
    let dir = TestDir::new();
    let lf = b"FIX:subject.\nbody \n\nRefs:#123\n";
    let crlf = b"FIX:subject.\r\nbody \r\n\r\nRefs:#123\r\n";
    let first = dir.command(&[], lf);
    let second = dir.command(&[], crlf);
    assert_eq!(first.status.code(), Some(1));
    assert_eq!(first.stderr, second.stderr);
    let messages: Vec<_> = stderr(&first).lines().collect();
    assert!(messages[0].contains(":1: error[header-format]"));
    assert!(messages[1].contains(":2: error[line-trailing-whitespace]"));
    assert!(messages[2].contains(":2: error[section-separator]"));
    assert!(messages[3].contains(":4: error[trailer-format]"));
}

const STRIP: &str = "COMMIT_MSG_LINT_STRIP_COMMENTS";
const CUT: &str = "COMMIT_MSG_LINT_CUT_SCISSORS";
const SCISSORS: &str = "# ------------------------ >8 ------------------------\n";

#[test]
fn each_filter_accepts_all_boolean_spellings_independently() {
    let dir = TestDir::new();
    let comment = b"# template\nfeat: change\n";
    let scissors = format!("feat: change\n\n{SCISSORS}bad trailing \n");
    for (name, other, input) in [
        (STRIP, CUT, &comment[..]),
        (CUT, STRIP, scissors.as_bytes()),
    ] {
        for (value, enabled) in [
            ("1", true),
            ("true", true),
            ("yes", true),
            ("on", true),
            ("TrUe", true),
            ("YeS", true),
            ("ON", true),
            ("0", false),
            ("false", false),
            ("no", false),
            ("off", false),
            ("FaLsE", false),
            ("No", false),
            ("OFF", false),
        ] {
            let output = dir.command_with_env(&[], input, &[(name, value), (other, "0")]);
            assert_eq!(
                output.status.code(),
                Some(if enabled { 0 } else { 1 }),
                "{name}={value}: {}",
                stderr(&output)
            );
        }
        let output = dir.command_with_env(&[], input, &[(other, "0")]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "unspecified {name}: {}",
            stderr(&output)
        );
        for value in ["", " ", " true", "true ", "2", "enabled", "true\n"] {
            let output = dir.command_with_env(&[], input, &[(name, value)]);
            assert_eq!(output.status.code(), Some(2), "{name}={value:?}");
            assert!(stderr(&output).contains(name));
        }
    }
}

#[test]
fn all_filter_combinations_apply_to_stdin_and_edit_without_changing_the_file() {
    let dir = TestDir::new();
    let input = format!("feat: change\n\n# note \n{SCISSORS}bad trailing \n");
    fs::write(dir.0.join("message"), &input).unwrap();
    for strip in [false, true] {
        for cut in [false, true] {
            let env = [
                (STRIP, if strip { "1" } else { "0" }),
                (CUT, if cut { "1" } else { "0" }),
            ];
            for edit in [false, true] {
                let args: &[&str] = if edit { &["--edit", "message"] } else { &[] };
                let output = dir.command_with_env(args, input.as_bytes(), &env);
                assert_eq!(
                    output.status.code(),
                    Some(if strip && cut { 0 } else { 1 }),
                    "strip={strip}, cut={cut}, edit={edit}: {}",
                    stderr(&output)
                );
                assert_eq!(
                    stderr(&output).contains(":3: error[line-trailing-whitespace]"),
                    !strip
                );
                assert_eq!(
                    stderr(&output).contains(":5: error[line-trailing-whitespace]"),
                    !cut
                );
                assert_eq!(fs::read(dir.0.join("message")).unwrap(), input.as_bytes());
            }
        }
    }
}

#[test]
fn git_prefix_respects_alias_order_includes_and_inherited_command_configuration() {
    let dir = TestDir::new();
    dir.git(&["init", "-q"]);
    fs::write(
        dir.0.join("included.gitconfig"),
        "[core]\n commentString = !!\n",
    )
    .unwrap();
    dir.git(&["config", "--add", "core.commentChar", "#"]);
    dir.git(&["config", "--add", "include.path", "../included.gitconfig"]);
    let input = b"!! template\nfeat: change\n";
    let output = dir.command(&[], input);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    fs::OpenOptions::new()
        .append(true)
        .open(dir.0.join(".git/config"))
        .unwrap()
        .write_all(b"\n[core]\n commentChar = \";\"\n")
        .unwrap();
    let output = dir.command(&[], b"; template\nfeat: change\n");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let output = dir.command_with_env(
        &[],
        input,
        &[
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "core.commentString"),
            ("GIT_CONFIG_VALUE_0", "!!"),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    dir.git(&["config", "--add", "core.commentString", " ! "]);
    let output = dir.command(&[], b" ! template\nfeat: change\n");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn git_config_environment_is_inherited_and_only_the_selected_prefix_is_decoded() {
    let dir = TestDir::new();
    dir.git(&["init", "-q"]);
    dir.git(&["config", "core.commentChar", ";"]);
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let alternate = dir.0.join("alternate-git-config");
    let env = [("GIT_CONFIG", alternate.to_str().unwrap())];
    fs::write(&alternate, b"").unwrap();
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"# template\nfeat: change\n",
        &env,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "empty Git configuration: {}",
        stderr(&output)
    );
    fs::write(
        &alternate,
        b"[unrelated]\n noValue\n empty =\n value = \xff\n[.extension]\n key = value\n[core]\n commentString = !!\n",
    )
    .unwrap();
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"!! template\nfeat: change\n",
        &env,
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    fs::write(&alternate, b"[unrelated]\n value = \xff\n").unwrap();
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"# template\nfeat: change\n",
        &env,
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    for value in [
        &b"commentString"[..],
        &b"commentString ="[..],
        &b"commentString = \xff"[..],
        &b"commentString = AUTO"[..],
    ] {
        let mut contents = b"[core]\n ".to_vec();
        contents.extend_from_slice(value);
        contents.push(b'\n');
        fs::write(&alternate, contents).unwrap();
        let output = dir.command_with_env(&["--config", "rules.jsonc"], b"feat: change\n", &env);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{value:?}: {}",
            stderr(&output)
        );
        assert!(stderr(&output).contains("comment prefix"));
    }
}

#[test]
fn outside_a_repository_uses_builtin_prefix_even_with_git_configuration() {
    let dir = TestDir::new();
    let output = dir.command_with_env(
        &[],
        b"# template\nfeat: change\n",
        &[
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "core.commentString"),
            ("GIT_CONFIG_VALUE_0", "!!"),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn explicit_git_dir_is_inherited_instead_of_inferring_repository_from_edit_path() {
    let dir = TestDir::new();
    let repo = dir.0.join("repo");
    fs::create_dir(&repo).unwrap();
    let output = dir
        .isolated_command("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let output = dir
        .isolated_command("git")
        .args(["config", "core.commentString", "!!"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    fs::write(dir.0.join("message"), "!! template\nfeat: change\n").unwrap();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let git_dir = repo.join(".git");
    let output = dir.command_with_env(
        &["--config", "rules.jsonc", "--edit", "message"],
        b"",
        &[("GIT_DIR", git_dir.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn filtered_diagnostics_keep_source_lines_width_severity_order_and_strict_behavior() {
    let dir = TestDir::new();
    let warning = format!("# intro\nfeat: {}\n", "界".repeat(23));
    for (args, code) in [(&[][..], 0), (&["--strict"][..], 1)] {
        let output = dir.command(args, warning.as_bytes());
        assert_eq!(output.status.code(), Some(code), "{}", stderr(&output));
        assert!(stderr(&output).contains("<stdin>:2: warning[header-max-width]"));
        assert_eq!(stderr(&output).lines().count(), 1);
    }
    let output = dir.command(&[], b"# intro\nFIX:subject.\n# note\nbody \n\nRefs:#123\n");
    assert_eq!(output.status.code(), Some(1));
    let lines: Vec<_> = stderr(&output).lines().collect();
    assert!(lines[0].contains(":2: error[header-format]"));
    assert!(lines[1].contains(":4: error[line-trailing-whitespace]"));
    assert!(lines[2].contains(":4: error[section-separator]"));
    assert!(lines[3].contains(":6: error[trailer-format]"));
}

#[test]
fn bad_git_prefix_or_configuration_fails_with_guidance_and_can_be_bypassed() {
    let dir = TestDir::new();
    dir.git(&["init", "-q"]);
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    for value in ["auto", "AuTo", "", "multi\nline"] {
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"feat: change\n",
            &[
                ("GIT_CONFIG_COUNT", "1"),
                ("GIT_CONFIG_KEY_0", "core.commentString"),
                ("GIT_CONFIG_VALUE_0", value),
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(2),
            "{value:?}: {}",
            stderr(&output)
        );
        assert!(stderr(&output).contains("fixed Git comment prefix"));
        assert!(stderr(&output).contains(STRIP) && stderr(&output).contains(CUT));
    }
    fs::write(dir.0.join(".git/config"), "invalid git configuration\n").unwrap();
    let output = dir.command(&["--config", "rules.jsonc"], b"feat: change\n");
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Git"));
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"feat: change\n",
        &[(STRIP, "off"), (CUT, "off")],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn missing_git_is_an_error_unless_both_filters_are_disabled_with_explicit_config() {
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let env = [("PATH", "")];
    let output = dir.command_with_env(&["--config", "rules.jsonc"], b"feat: change\n", &env);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Git"));
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"feat: change\n",
        &[("PATH", ""), (STRIP, "0"), (CUT, "0")],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn git_trace_does_not_change_default_prefix_resolution_inside_or_outside_a_repository() {
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    for bare in ["0", "1"] {
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"# template\nfeat: change\n",
            &[("GIT_TRACE", "1"), ("GIT_TRACE_BARE", bare)],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "outside repository: {}",
            stderr(&output)
        );
    }
    dir.git(&["init", "-q"]);
    for bare in ["0", "1"] {
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"# template\nfeat: change\n",
            &[("GIT_TRACE", "1"), ("GIT_TRACE_BARE", bare)],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "unset prefix: {}",
            stderr(&output)
        );
    }
    dir.git(&["config", "core.commentString", "!!"]);
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"!! template\nfeat: change\n",
        &[("GIT_TRACE", "1")],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "configured prefix: {}",
        stderr(&output)
    );
}

#[test]
fn git_trace2_modes_preserve_prefix_resolution_and_real_errors() {
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    for key in ["GIT_TRACE2", "GIT_TRACE2_EVENT", "GIT_TRACE2_PERF"] {
        for brief in ["0", "1"] {
            let env = [(key, "1"), ("GIT_TRACE2_BRIEF", brief)];
            let output = dir.command_with_env(
                &["--config", "rules.jsonc"],
                b"# template\nfeat: change\n",
                &env,
            );
            assert_eq!(
                output.status.code(),
                Some(0),
                "outside {key}, brief={brief}: {}",
                stderr(&output)
            );
        }
    }
    dir.git(&["init", "-q"]);
    for key in ["GIT_TRACE2", "GIT_TRACE2_EVENT", "GIT_TRACE2_PERF"] {
        for brief in ["0", "1"] {
            let env = [(key, "1"), ("GIT_TRACE2_BRIEF", brief)];
            let output = dir.command_with_env(
                &["--config", "rules.jsonc"],
                b"# template\nfeat: change\n",
                &env,
            );
            assert_eq!(
                output.status.code(),
                Some(0),
                "unset {key}, brief={brief}: {}",
                stderr(&output)
            );
            let output = dir.command_with_env(
                &["--config", "rules.jsonc"],
                b"feat: change\n",
                &[(key, "1"), ("GIT_CONFIG_COUNT", "invalid")],
            );
            assert_eq!(
                output.status.code(),
                Some(2),
                "error {key}: {}",
                stderr(&output)
            );
        }
    }
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"# template\nfeat: change\n",
        &[("GIT_TRACE2_PERF", "1"), ("GIT_TRACE2_PERF_BRIEF", "1")],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "brief perf: {}",
        stderr(&output)
    );
    let env = [
        ("GIT_TRACE", "1"),
        ("GIT_TRACE2", "1"),
        ("GIT_TRACE2_EVENT", "1"),
        ("GIT_TRACE2_PERF", "1"),
    ];
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"# template\nfeat: change\n",
        &env,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "combined traces: {}",
        stderr(&output)
    );
    dir.git(&["config", "core.commentString", "!!"]);
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"!! template\nfeat: change\n",
        &env,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "configured prefix: {}",
        stderr(&output)
    );
    let trace_file = dir.0.join("git-trace.json");
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"!! template\nfeat: change\n",
        &[("GIT_TRACE2_EVENT", trace_file.to_str().unwrap())],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "trace file: {}",
        stderr(&output)
    );
    assert!(
        fs::read_to_string(trace_file)
            .unwrap()
            .contains("\"event\"")
    );
}

#[test]
fn git_trace2_brief_from_git_config_and_nonzero_numeric_values_are_supported() {
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let global = dir.0.join("global-git-config");
    fs::write(
        &global,
        "[trace2]\n normalBrief = true\n perfBrief = true\n[demo]\n secret = private-value\n",
    )
    .unwrap();
    let global_path = global.to_str().unwrap();
    for env in [
        &[("GIT_TRACE2", "1"), ("GIT_CONFIG_GLOBAL", global_path)][..],
        &[("GIT_TRACE2_PERF", "1"), ("GIT_CONFIG_GLOBAL", global_path)][..],
        &[
            ("GIT_TRACE2", "1"),
            ("GIT_CONFIG_GLOBAL", global_path),
            ("GIT_TRACE2_CONFIG_PARAMS", "demo.*"),
        ][..],
        &[
            ("GIT_TRACE2_PERF", "1"),
            ("GIT_CONFIG_GLOBAL", global_path),
            ("GIT_TRACE2_CONFIG_PARAMS", "demo.*"),
        ][..],
        &[("GIT_TRACE2", "1"), ("GIT_TRACE2_BRIEF", "2")][..],
        &[("GIT_TRACE2", "1"), ("GIT_TRACE2_BRIEF", "-1")][..],
        &[("GIT_TRACE2_PERF", "1"), ("GIT_TRACE2_PERF_BRIEF", "2")][..],
    ] {
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"# template\nfeat: change\n",
            env,
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "outside {env:?}: {}",
            stderr(&output)
        );
    }
    dir.git(&["init", "-q"]);
    for env in [
        &[("GIT_TRACE2", "1"), ("GIT_CONFIG_GLOBAL", global_path)][..],
        &[("GIT_TRACE2_PERF", "1"), ("GIT_CONFIG_GLOBAL", global_path)][..],
        &[("GIT_TRACE2", "1"), ("GIT_TRACE2_BRIEF", "2")][..],
    ] {
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"# template\nfeat: change\n",
            env,
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "inside {env:?}: {}",
            stderr(&output)
        );
    }
}

#[cfg(unix)]
#[test]
fn git_dir_path_with_newline_is_accepted_and_its_configuration_used() {
    let dir = TestDir::new();
    let git_dir = dir.0.join("git\ndir");
    dir.git(&["init", "--bare", "-q", git_dir.to_str().unwrap()]);
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"# template\nfeat: change\n",
        &[
            ("GIT_DIR", git_dir.to_str().unwrap()),
            ("GIT_TRACE2", "1"),
            ("GIT_TRACE2_EVENT", "1"),
            ("GIT_TRACE2_PERF", "1"),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    dir.git(&[
        "--git-dir",
        git_dir.to_str().unwrap(),
        "config",
        "core.commentString",
        "!!",
    ]);
    let output = dir.command_with_env(
        &["--config", "rules.jsonc"],
        b"!! template\nfeat: change\n",
        &[("GIT_DIR", git_dir.to_str().unwrap()), ("GIT_TRACE", "1")],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[cfg(unix)]
#[test]
fn filesystem_boundary_discovery_is_outside_a_repository_but_other_failures_are_errors() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let git = dir.0.join("git");
    for (diagnostic, expected) in [
        (
            "fatal: not a git repository (or any parent up to mount point /dev)\\nStopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).",
            0,
        ),
        (
            "fatal: not a git repository (or any parent up to mount point /dev)\\nerror: unrelated failure",
            2,
        ),
        ("fatal: unrelated failure", 2),
    ] {
        fs::write(
            &git,
            format!("#!/bin/sh\nprintf '%b\\n' '{diagnostic}' >&2\nexit 128\n"),
        )
        .unwrap();
        fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"# template\nfeat: change\n",
            &[("PATH", dir.0.to_str().unwrap())],
        );
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{diagnostic}: {}",
            stderr(&output)
        );
    }
}

#[cfg(unix)]
#[test]
fn actual_filesystem_boundary_falls_back_when_available() {
    let dir = TestDir::new();
    let config = dir.0.join("rules.jsonc");
    fs::write(&config, r#"{ "version": 1 }"#).unwrap();
    let global = dir.0.join("global-git-config");
    fs::write(
        &global,
        "[trace2]\n normalBrief = true\n perfBrief = true\n",
    )
    .unwrap();
    let global_path = global.to_str().unwrap();
    for path in ["/dev", "/dev/shm", "/proc"] {
        let output = dir
            .isolated_command("git")
            .env_remove("GIT_CEILING_DIRECTORIES")
            .env("LC_ALL", "C")
            .current_dir(path)
            .args(["rev-parse", "--git-dir"])
            .output();
        let Ok(output) = output else { continue };
        if !stderr(&output)
            .contains("Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).")
        {
            continue;
        }
        for env in [
            &[("GIT_TRACE", "1")][..],
            &[("GIT_TRACE2", "1")][..],
            &[("GIT_TRACE2", "1"), ("GIT_TRACE2_BRIEF", "1")][..],
            &[("GIT_TRACE2_EVENT", "1")][..],
            &[("GIT_TRACE2_PERF", "1")][..],
            &[("GIT_TRACE2_PERF", "1"), ("GIT_TRACE2_PERF_BRIEF", "1")][..],
            &[("GIT_TRACE2", "1"), ("GIT_TRACE2_BRIEF", "2")][..],
            &[("GIT_TRACE2", "1"), ("GIT_CONFIG_GLOBAL", global_path)][..],
            &[("GIT_TRACE2_PERF", "1"), ("GIT_CONFIG_GLOBAL", global_path)][..],
            &[
                ("GIT_TRACE2", "1"),
                ("GIT_TRACE2_EVENT", "1"),
                ("GIT_TRACE2_PERF", "1"),
            ][..],
        ] {
            let mut child = dir
                .isolated_command(env!("CARGO_BIN_EXE_commit-msg-lint"))
                .env_remove("GIT_CEILING_DIRECTORIES")
                .envs(env.iter().copied())
                .current_dir(path)
                .arg("--config")
                .arg(&config)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"# template\nfeat: change\n")
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{path} {env:?}: {}",
                stderr(&output)
            );
        }
        return;
    }
    eprintln!(
        "no accessible filesystem boundary without a repository; synthetic test covers this case"
    );
}

#[test]
fn invalid_explicit_git_directory_is_not_mistaken_for_a_directory_outside_a_repository() {
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    for env in [
        &[("GIT_DIR", "missing")][..],
        &[("GIT_DIR", "missing"), ("GIT_TRACE", "1")][..],
    ] {
        let output = dir.command_with_env(&["--config", "rules.jsonc"], b"feat: change\n", env);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(stderr(&output).contains("Git"));
    }
}

#[cfg(unix)]
#[test]
fn non_unicode_filter_environment_is_a_usage_error() {
    use std::os::unix::ffi::OsStringExt;
    let dir = TestDir::new();
    for name in [STRIP, CUT] {
        let output = dir
            .isolated_command(env!("CARGO_BIN_EXE_commit-msg-lint"))
            .env(name, std::ffi::OsString::from_vec(vec![0xff]))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains(name));
    }
}

#[cfg(unix)]
#[test]
fn git_configuration_failures_do_not_expose_unrelated_output_or_trace_values() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let git = dir.0.join("git");
    for script in [
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else printf 'unrelated.secret\\nprivate-value\\0'; echo 'secret trace private-value' >&2; exit 1; fi",
        "echo 'secret trace private-value' >&2; exit 128",
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else printf 'unrelated.secret\\nprivate-value\\0invalid-key\\0'; fi",
    ] {
        fs::write(&git, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"feat: change\n",
            &[("PATH", dir.0.to_str().unwrap())],
        );
        assert_eq!(
            output.status.code(),
            Some(2),
            "{script}: {}",
            stderr(&output)
        );
        assert!(!stderr(&output).contains("private-value"));
        assert!(!stderr(&output).contains("unrelated.secret"));
    }
}

#[cfg(unix)]
#[test]
fn malformed_git_output_and_git_failures_are_not_treated_as_unset_prefixes() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TestDir::new();
    fs::write(dir.0.join("rules.jsonc"), r#"{ "version": 1 }"#).unwrap();
    let git = dir.0.join("git");
    for script in [
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else printf 'core.commentchar\\n#'; fi",
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else printf 'wrongkey\\n#\\0'; fi",
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else echo failure >&2; exit 1; fi",
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else printf 'trace: doing work\\nerror: real failure\\n' >&2; exit 1; fi",
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else printf '%s\\n' '{\"event\":\"error\",\"sid\":\"s\",\"thread\":\"main\",\"msg\":\"real failure\"}' >&2; exit 1; fi",
        "printf '%s\\n' '{\"event\":\"error\",\"sid\":\"s\",\"thread\":\"main\",\"msg\":\"fatal: not a git repository (or any of the parent directories): .git\"}' >&2; exit 128",
        "printf '%s\\n' '{\"event\":\"version\",\"sid\":\"s\",\"thread\":\"main\",\"msg\":\"fatal: not a git repository (or any of the parent directories): .git\"}' >&2; exit 128",
        "printf '%s\\n' '{\"event\":\"error\",\"sid\":\"s\",\"thread\":\"main\",\"msg\":\"unrelated\"}' 'fatal: not a git repository (or any of the parent directories): .git' >&2; exit 128",
        "if [ \"$1\" = rev-parse ]; then printf '.git\\n'; else exit 2; fi",
        "if [ \"$1\" = rev-parse ]; then printf ''; else exit 0; fi",
        "echo broken >&2; exit 128",
    ] {
        fs::write(&git, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
        let output = dir.command_with_env(
            &["--config", "rules.jsonc"],
            b"feat: change\n",
            &[("PATH", dir.0.to_str().unwrap())],
        );
        assert_eq!(
            output.status.code(),
            Some(2),
            "{script}: {}",
            stderr(&output)
        );
        assert!(stderr(&output).contains("fixed Git comment prefix"));
    }
}

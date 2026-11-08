use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

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

    fn command(&self, args: &[&str], input: &[u8]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_commit-msg-lint"))
            .current_dir(&self.0)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
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
    fs::write(dir.0.join("rules.toml"), "version = 1\n").unwrap();

    let output = dir.command(&["--config=rules.toml", "--edit=message"], b"ignored");

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
}

#[test]
fn explicit_configuration_and_edit_path_are_used() {
    let dir = TestDir::new();
    fs::write(dir.0.join("message"), "feat(cli): change\n").unwrap();
    fs::write(
        dir.0.join("rules.toml"),
        "version = 1\n[conventional]\nallowed-scopes = []\n",
    )
    .unwrap();
    let output = dir.command(&["--config", "rules.toml", "--edit", "message"], b"ignored");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("message:1: error[scope-enum]"));
    fs::write(
        dir.0.join("rules.toml"),
        "version = 1\n[conventional]\nallowed-scopes = []\n[rules.scope-enum]\nenabled = false\n",
    )
    .unwrap();
    assert_eq!(
        dir.command(&["--config", "rules.toml", "--edit", "message"], b"")
            .status
            .code(),
        Some(0)
    );
}

#[test]
fn repository_root_configuration_is_discovered_from_subdirectories() {
    let dir = TestDir::new();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&dir.0)
            .status()
            .unwrap()
            .success()
    );
    fs::write(
        dir.0.join("commit-msg-lint.toml"),
        "version = 1\n[conventional]\ntypes = ['fix']\n",
    )
    .unwrap();
    fs::create_dir(dir.0.join("child")).unwrap();
    fs::write(dir.0.join("child/commit-msg-lint.toml"), "not valid toml").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_commit-msg-lint"))
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
fn malformed_inputs_and_configuration_are_errors() {
    let dir = TestDir::new();
    fs::write(dir.0.join("bad.toml"), "version = 1\nunknown = 1\n").unwrap();
    fs::write(dir.0.join("bad-utf8"), [0xff]).unwrap();
    for (args, input) in [
        (vec!["--unknown"], &b"feat: change"[..]),
        (vec!["--edit"], &b""[..]),
        (vec!["--edit", "bad-utf8"], &b""[..]),
        (vec!["--edit", "missing"], &b""[..]),
        (vec!["--config", "bad.toml"], &b"feat: change"[..]),
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

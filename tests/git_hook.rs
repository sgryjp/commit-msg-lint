use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const STRIP: &str = "COMMIT_MSG_LINT_STRIP_COMMENTS";
const CUT: &str = "COMMIT_MSG_LINT_CUT_SCISSORS";
const MARKER: &str = "# ------------------------ >8 ------------------------";

struct HookRepo {
    _temp: TempDir,
    root: PathBuf,
    repo: PathBuf,
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// Git for Windows runs hooks and editors through its bundled POSIX shell.
fn script(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn isolated_command(program: impl AsRef<OsStr>, root: &Path, cwd: &Path) -> Command {
    let mut command = Command::new(program);
    command.env_clear().current_dir(cwd);
    // These are needed to locate Git, its shell, and Windows system libraries.
    for key in [
        "PATH",
        "PATHEXT",
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "TMP",
        "TEMP",
        "TMPDIR",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("xdg"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", root.join("global-config"))
        .env("GIT_CEILING_DIRECTORIES", root.parent().unwrap())
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_AUTHOR_NAME", "Test Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test Committer")
        .env("GIT_COMMITTER_EMAIL", "committer@example.invalid")
        .env("LC_ALL", "C")
        .env("GIT_EDITOR", "sh \"$TEST_EDITOR\"")
        .env("TEST_EDITOR", root.join("editor"))
        .env("TEST_MESSAGE", root.join("message"))
        .env("HOOK_CAPTURE", root.join("hook-input"))
        .env("HOOK_AFTER", root.join("hook-after"))
        .env("HOOK_STATUS", root.join("hook-status"))
        .env("LINT_BIN", env!("CARGO_BIN_EXE_commit-msg-lint"))
        .env("LINT_CONFIG", root.join("lint.toml"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

impl HookRepo {
    fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("commit-msg-hook-")
            .tempdir()
            .unwrap();
        let root = temp.path().to_path_buf();
        let repo = root.join("repo");
        fs::create_dir(&repo).unwrap();
        fs::create_dir(root.join("empty-template")).unwrap();
        fs::create_dir(root.join("hooks")).unwrap();
        fs::write(root.join("global-config"), "").unwrap();
        fs::write(root.join("lint.toml"), "version = 1\n").unwrap();
        script(
            &root.join("editor"),
            "#!/bin/sh\nset -e\nif [ \"${EDITOR_MODE:-}\" = prepend ]; then\n  printf 'feat: change\\n\\n' > \"$1.tmp\"\n  cat \"$1\" >> \"$1.tmp\"\n  mv \"$1.tmp\" \"$1\"\nelse\n  cp \"$TEST_MESSAGE\" \"$1\"\nfi\n",
        );
        script(
            &root.join("hooks/commit-msg"),
            "#!/bin/sh\ncp \"$1\" \"$HOOK_CAPTURE\" || exit 99\nif [ -n \"${HOOK_BAD_GLOBAL:-}\" ]; then\n  GIT_CONFIG_GLOBAL=$HOOK_BAD_GLOBAL\n  export GIT_CONFIG_GLOBAL\nfi\nif [ -n \"${HOOK_BAD_CONFIG:-}\" ]; then\n  GIT_CONFIG=$HOOK_BAD_CONFIG\n  export GIT_CONFIG\nfi\n\"$LINT_BIN\" --config \"$LINT_CONFIG\" --edit \"$1\"\nstatus=$?\ncp \"$1\" \"$HOOK_AFTER\" || exit 99\nprintf '%s\\n' \"$status\" > \"$HOOK_STATUS\"\nexit \"$status\"\n",
        );
        let this = Self {
            _temp: temp,
            root,
            repo,
        };
        let template = this.root.join("empty-template");
        assert_success(&this.git_in(
            &this.repo,
            &["init", "-q", "--template", template.to_str().unwrap()],
            &[],
        ));
        for (key, value) in [
            (
                "core.hooksPath",
                this.root.join("hooks").to_str().unwrap().to_owned(),
            ),
            ("core.autocrlf", "false".into()),
            ("core.safecrlf", "false".into()),
            ("commit.gpgSign", "false".into()),
        ] {
            this.config(key, &value);
        }
        fs::write(this.repo.join("tracked"), format!("{} \n", "x".repeat(100))).unwrap();
        assert_success(&this.git(&["add", "tracked"], &[]));
        this
    }

    fn git_in(&self, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        isolated_command("git", &self.root, cwd)
            .args(args)
            .envs(env.iter().copied())
            .output()
            .unwrap()
    }

    fn git(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.git_in(&self.repo, args, env)
    }

    fn config(&self, key: &str, value: &str) {
        assert_success(&self.git(&["config", "--local", key, value], &[]));
    }

    fn message(&self, message: impl AsRef<[u8]>) {
        fs::write(self.root.join("message"), message).unwrap();
    }

    fn captured(&self) -> Vec<u8> {
        fs::read(self.root.join("hook-input")).expect("commit-msg hook did not run")
    }

    fn hook_status(&self) -> i32 {
        fs::read_to_string(self.root.join("hook-status"))
            .expect("commit-msg hook did not finish")
            .trim()
            .parse()
            .unwrap()
    }

    fn final_message(&self, cwd: &Path) -> String {
        let output = self.git_in(cwd, &["log", "-1", "--format=%B"], &[]);
        assert_success(&output);
        text(&output.stdout).to_owned()
    }

    fn supports_comment_string(&self) -> bool {
        let mut child = isolated_command("git", &self.root, &self.repo)
            .args([
                "-c",
                "core.commentString=//",
                "stripspace",
                "--strip-comments",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"// comment\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        output.status.success() && output.stdout.is_empty()
    }
}

#[test]
fn editor_commit_strips_git_comments_and_preserves_a_valid_header() {
    let repo = HookRepo::new();
    repo.message("feat: change\n\n# ignored \n");
    let output = repo.git(&["commit", "-e", "--cleanup=strip", "-q"], &[]);
    assert_success(&output);
    assert_eq!(repo.hook_status(), 0);
    assert!(text(&repo.captured()).contains("# ignored "));
    assert_eq!(
        repo.captured(),
        fs::read(repo.root.join("hook-after")).unwrap()
    );
    assert_eq!(repo.final_message(&repo.repo), "feat: change\n\n");
}

#[test]
fn hook_diagnostics_use_original_line_numbers_and_do_not_rewrite_the_message() {
    let repo = HookRepo::new();
    let message = "# preface\nfeat: change\n\n# ignored\nbody \n";
    repo.message(message);
    let output = repo.git(&["commit", "-e", "--cleanup=verbatim", "-q"], &[]);
    assert!(!output.status.success());
    assert_eq!(repo.hook_status(), 1);
    assert!(
        text(&output.stderr).contains(":5: error[line-trailing-whitespace]"),
        "{}",
        text(&output.stderr)
    );
    assert_eq!(repo.captured(), message.as_bytes());
    assert_eq!(
        repo.captured(),
        fs::read(repo.root.join("hook-after")).unwrap()
    );
}

#[test]
fn verbose_editor_generated_scissors_cuts_git_diff_as_a_canary() {
    let repo = HookRepo::new();
    let output = repo.git(&["commit", "-v", "-q"], &[("EDITOR_MODE", "prepend")]);
    assert_success(&output);
    assert_eq!(repo.hook_status(), 0);
    let captured = repo.captured();
    let input = text(&captured);
    assert!(input.lines().any(|line| line == MARKER), "{input}");
    assert!(input.contains("diff --git"), "{input}");
    assert_eq!(repo.final_message(&repo.repo), "feat: change\n\n");

    let repo = HookRepo::new();
    let output = repo.git(
        &["commit", "-v", "-q"],
        &[("EDITOR_MODE", "prepend"), (CUT, "false")],
    );
    assert!(!output.status.success());
    assert_eq!(repo.hook_status(), 1);
    assert!(
        text(&output.stderr).contains("line-trailing-whitespace"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn message_and_file_input_can_retain_comments_that_the_hook_ignores() {
    for source in ["-m", "-F"] {
        let repo = HookRepo::new();
        let message = format!("feat: change\n\n#{}\n", "x".repeat(80));
        repo.message(&message);
        let message_file = repo.root.join("message");
        let arg = if source == "-m" {
            message.as_str()
        } else {
            message_file.to_str().unwrap()
        };
        let output = repo.git(&["commit", "-q", source, arg], &[]);
        assert_success(&output);
        assert_eq!(repo.hook_status(), 0);
        assert!(
            repo.final_message(&repo.repo)
                .contains(&format!("#{}", "x".repeat(80)))
        );
        assert_eq!(
            fs::read(repo.root.join("message")).unwrap(),
            message.as_bytes()
        );
    }
}

#[test]
fn cleanup_modes_and_policy_overrides_are_independent() {
    for cleanup in ["strip", "whitespace", "scissors", "verbatim"] {
        for strip in [true, false] {
            let repo = HookRepo::new();
            let message = format!("feat: change\n\n#{}\n", "x".repeat(80));
            repo.message(&message);
            let cleanup_arg = format!("--cleanup={cleanup}");
            let output = repo.git(
                &["commit", "-e", "-q", &cleanup_arg],
                &[
                    (STRIP, if strip { "true" } else { "false" }),
                    (CUT, "false"),
                ],
            );
            assert_eq!(
                repo.hook_status(),
                if strip { 0 } else { 1 },
                "{cleanup}: {}",
                text(&output.stderr)
            );
            assert_eq!(
                output.status.success(),
                strip,
                "{cleanup}: {}",
                text(&output.stderr)
            );
            if strip {
                assert_eq!(
                    repo.final_message(&repo.repo)
                        .contains(&format!("#{}", "x".repeat(80))),
                    cleanup != "strip",
                    "cleanup={cleanup}"
                );
            } else {
                assert!(text(&output.stderr).contains("body-max-width"));
            }
        }
    }
}

#[test]
fn scissors_cut_can_differ_from_gits_final_verbatim_message() {
    for cut in [true, false] {
        let repo = HookRepo::new();
        let message = format!("feat: change\n\n{MARKER}\n{}\n", "x".repeat(80));
        repo.message(&message);
        let output = repo.git(
            &["commit", "-e", "--cleanup=verbatim", "-q"],
            &[(STRIP, "false"), (CUT, if cut { "true" } else { "false" })],
        );
        assert_eq!(
            repo.hook_status(),
            if cut { 0 } else { 1 },
            "{}",
            text(&output.stderr)
        );
        assert_eq!(output.status.success(), cut);
        if cut {
            assert!(repo.final_message(&repo.repo).contains(&message));
        } else {
            assert!(text(&output.stderr).contains("body-max-width"));
        }
    }
}

#[test]
fn scissors_cleanup_and_cut_policy_can_be_aligned() {
    for cut in [true, false] {
        let repo = HookRepo::new();
        let message = format!("feat: change\n\n{MARKER}\n{}\n", "x".repeat(80));
        repo.message(&message);
        let output = repo.git(
            &["commit", "-e", "--cleanup=scissors", "-q"],
            &[(STRIP, "false"), (CUT, if cut { "true" } else { "false" })],
        );
        assert_eq!(
            repo.hook_status(),
            if cut { 0 } else { 1 },
            "{}",
            text(&output.stderr)
        );
        assert_eq!(output.status.success(), cut);
        if cut {
            assert_eq!(repo.final_message(&repo.repo), "feat: change\n\n");
        } else {
            assert!(text(&output.stderr).contains("body-max-width"));
        }
    }
}

#[test]
fn only_an_exact_lf_terminated_scissors_line_cuts_the_input() {
    let variants = [
        (format!("{MARKER}\n"), true),
        (format!(" {MARKER}\n"), false),
        (format!("{MARKER} extra\n"), false),
        (format!("{MARKER}\r\n"), false),
        (MARKER.to_owned(), false),
    ];
    for (marker, recognized) in variants {
        let repo = HookRepo::new();
        fs::write(repo.root.join("lint.toml"), "version = 1\n[rules.body-max-width]\nthresholds = [{limit = 20, severity = 'error'}]\n").unwrap();
        let message = format!("feat: change\n\n{marker}");
        repo.message(&message);
        let output = repo.git(
            &["commit", "-e", "--cleanup=verbatim", "-q"],
            &[(STRIP, "false")],
        );
        assert_eq!(
            repo.captured(),
            message.as_bytes(),
            "Git changed hook input for {marker:?}"
        );
        assert_eq!(
            repo.hook_status(),
            if recognized { 0 } else { 1 },
            "{marker:?}: {}",
            text(&output.stderr)
        );
        assert_eq!(
            output.status.success(),
            recognized,
            "{marker:?}: {}",
            text(&output.stderr)
        );
        if !recognized {
            assert!(text(&output.stderr).contains("body-max-width"));
        }
    }
}

#[test]
fn configured_prefix_include_and_command_line_configuration_reach_the_hook() {
    for case in ["default", "changed", "include", "command-line"] {
        let repo = HookRepo::new();
        let prefix = match case {
            "default" => "#",
            "changed" => {
                repo.config("core.commentChar", ";");
                ";"
            }
            "include" => {
                fs::write(
                    repo.root.join("included-config"),
                    "[core]\n\tcommentChar = \";\"\n",
                )
                .unwrap();
                repo.config(
                    "include.path",
                    repo.root.join("included-config").to_str().unwrap(),
                );
                ";"
            }
            _ => ";",
        };
        let message = format!("feat: change\n\n{prefix}{}\n", "x".repeat(80));
        let mut args = vec![];
        if case == "command-line" {
            args.extend(["-c", "core.commentChar=;"]);
        }
        args.extend(["commit", "-m", &message, "-q"]);
        let output = repo.git(&args, &[]);
        assert_success(&output);
        assert_eq!(repo.hook_status(), 0, "{case}: {}", text(&output.stderr));
        assert!(
            repo.final_message(&repo.repo)
                .contains(&format!("{prefix}{}", "x".repeat(80)))
        );
    }
}

#[test]
fn multichar_prefix_and_last_alias_setting_are_honored() {
    let probe = HookRepo::new();
    if !probe.supports_comment_string() {
        eprintln!("Git lacks core.commentString support; multi-character/alias cases skipped");
        return;
    }
    for reversed in [false, true] {
        let repo = HookRepo::new();
        if reversed {
            repo.config("core.commentString", "//");
            repo.config("core.commentChar", ";");
        } else {
            repo.config("core.commentChar", ";");
            repo.config("core.commentString", "//");
        }
        let prefix = if reversed { ";" } else { "//" };
        let message = format!("feat: change\n\n{prefix}{}\n", "x".repeat(80));
        let output = repo.git(&["commit", "-m", &message, "-q"], &[]);
        assert_success(&output);
        assert_eq!(repo.hook_status(), 0);
        assert!(repo.final_message(&repo.repo).contains(prefix));
    }
}

#[test]
fn multichar_prefix_is_used_for_scissors_when_comment_stripping_is_disabled() {
    let repo = HookRepo::new();
    if !repo.supports_comment_string() {
        eprintln!("Git lacks core.commentString support; multi-character scissors case skipped");
        return;
    }
    repo.config("core.commentString", "//");
    let message = format!(
        "feat: change\n\n// ------------------------ >8 ------------------------\n{}\n",
        "x".repeat(80)
    );
    repo.message(&message);
    let output = repo.git(
        &["commit", "-e", "--cleanup=verbatim", "-q"],
        &[(STRIP, "false")],
    );
    assert_success(&output);
    assert_eq!(repo.hook_status(), 0);
    assert!(repo.final_message(&repo.repo).contains(&message));
}

#[test]
fn linked_worktree_configuration_is_used_in_the_hook() {
    let repo = HookRepo::new();
    assert_success(&repo.git(&["commit", "-m", "feat: initial", "-q"], &[]));
    assert_success(&repo.git(&["config", "extensions.worktreeConfig", "true"], &[]));
    let worktree = repo.root.join("worktree");
    assert_success(&repo.git(
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "test-worktree",
            worktree.to_str().unwrap(),
        ],
        &[],
    ));
    assert_success(&repo.git_in(
        &worktree,
        &["config", "--worktree", "core.commentChar", ";"],
        &[],
    ));
    fs::write(worktree.join("new-file"), "new\n").unwrap();
    assert_success(&repo.git_in(&worktree, &["add", "new-file"], &[]));
    let message = format!("feat: worktree\n\n;{}\n", "x".repeat(80));
    fs::remove_file(repo.root.join("hook-status")).unwrap();
    let output = repo.git_in(&worktree, &["commit", "-q", "-m", &message], &[]);
    assert_success(&output);
    assert_eq!(repo.hook_status(), 0);
    assert!(text(&repo.captured()).contains(&message));
    assert!(repo.final_message(&worktree).contains(';'));
}

#[test]
fn invalid_git_configuration_in_the_hook_is_a_configuration_error() {
    for contents in [
        "[core]\ncommentChar = auto\n",
        "[core]\ncommentChar =\n",
        "[core\nmalformed\n",
    ] {
        let repo = HookRepo::new();
        let bad = repo.root.join("bad-global");
        fs::write(&bad, contents).unwrap();
        let output = repo.git(
            &["commit", "-q", "-m", "feat: change"],
            &[("HOOK_BAD_GLOBAL", bad.to_str().unwrap())],
        );
        assert!(!output.status.success());
        assert_eq!(
            repo.hook_status(),
            2,
            "{contents:?}: {}",
            text(&output.stderr)
        );
        let diagnostic = text(&output.stderr);
        assert!(diagnostic.contains("fixed"), "{diagnostic}");
        assert!(
            diagnostic.contains(STRIP) && diagnostic.contains(CUT),
            "{diagnostic}"
        );

        let output = repo.git(
            &["commit", "-q", "-m", "feat: change"],
            &[
                ("HOOK_BAD_GLOBAL", bad.to_str().unwrap()),
                (STRIP, "off"),
                (CUT, "off"),
            ],
        );
        assert_success(&output);
        assert_eq!(repo.hook_status(), 0);
    }
}

#[test]
fn git_config_lookup_failure_in_the_hook_is_not_treated_as_an_unset_prefix() {
    let repo = HookRepo::new();
    let bad = repo.root.join("bad-config");
    fs::write(&bad, "[broken\n").unwrap();
    let path = bad.to_str().unwrap();
    assert_success(&repo.git(&["rev-parse", "--git-dir"], &[("GIT_CONFIG", path)]));
    assert!(
        !repo
            .git(
                &["config", "--get-regexp", "^core\\.comment(char|string)$"],
                &[("GIT_CONFIG", path)]
            )
            .status
            .success()
    );
    let output = repo.git(
        &["commit", "-q", "-m", "feat: change"],
        &[("HOOK_BAD_CONFIG", path)],
    );
    assert!(!output.status.success());
    assert_eq!(repo.hook_status(), 2, "{}", text(&output.stderr));
    let diagnostic = text(&output.stderr);
    assert!(
        diagnostic.contains("Git") && diagnostic.contains("fixed"),
        "{diagnostic}"
    );
}

#[test]
fn outside_a_repository_the_cli_uses_the_default_comment_prefix() {
    let repo = HookRepo::new();
    // A valid but unusable repository prefix must not override the fallback.
    fs::write(
        repo.root.join("global-config"),
        "[core]\ncommentChar = auto\n",
    )
    .unwrap();
    let mut child = isolated_command(
        env!("CARGO_BIN_EXE_commit-msg-lint"),
        &repo.root,
        &repo.root,
    )
    .args(["--config", repo.root.join("lint.toml").to_str().unwrap()])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("feat: change\n\n#{}\n", "x".repeat(80)).as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_success(&output);
}

use serde::Deserialize;
use std::env;
use std::process::{Command, Output};

const STRIP_COMMENTS: &str = "COMMIT_MSG_LINT_STRIP_COMMENTS";
const CUT_SCISSORS: &str = "COMMIT_MSG_LINT_CUT_SCISSORS";
const GUIDANCE: &str = "set a fixed Git comment prefix, or disable both COMMIT_MSG_LINT_STRIP_COMMENTS and COMMIT_MSG_LINT_CUT_SCISSORS";

pub struct FilterPolicy {
    pub strip_comments: bool,
    pub cut_scissors: bool,
}

impl FilterPolicy {
    pub fn from_environment() -> Result<Self, String> {
        Ok(Self {
            strip_comments: read_boolean(STRIP_COMMENTS)?,
            cut_scissors: read_boolean(CUT_SCISSORS)?,
        })
    }

    pub fn comment_prefix(&self) -> Result<String, String> {
        if !self.strip_comments && !self.cut_scissors {
            return Ok("#".to_owned());
        }
        resolve_comment_prefix().map_err(|error| format!("{error}; {GUIDANCE}"))
    }
}

fn read_boolean(name: &str) -> Result<bool, String> {
    let Some(value) = env::var_os(name) else {
        return Ok(true);
    };
    let value = value
        .to_str()
        .ok_or_else(|| format!("{name} must be Unicode"))?;
    if ["1", "true", "yes", "on"]
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
    {
        Ok(true)
    } else if ["0", "false", "no", "off"]
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
    {
        Ok(false)
    } else {
        Err(format!(
            "invalid {name}: expected 1/true/yes/on or 0/false/no/off"
        ))
    }
}

pub fn git_command() -> Command {
    let mut command = Command::new("git");
    // Preserve history mode's no-automatic-fetch contract in partial clones.
    command.env("GIT_NO_LAZY_FETCH", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

fn run_git(args: &[&str]) -> Result<Output, String> {
    git_command()
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("cannot run Git: {e}"))
}

fn resolve_comment_prefix() -> Result<String, String> {
    let repository = run_git(&["rev-parse", "--git-dir"])?;
    if !repository.status.success() {
        let stderr = String::from_utf8_lossy(&repository.stderr);
        if repository.status.code() == Some(128)
            && repository.stdout.is_empty()
            && env::var_os("GIT_DIR").is_none()
            && is_outside_repository(&stderr)
        {
            return Ok("#".to_owned());
        }
        return Err(format!(
            "cannot determine Git repository (Git exited with {})",
            repository.status
        ));
    }
    if !valid_repository_output(&repository.stdout) {
        return Err("invalid output from Git repository discovery".to_owned());
    }

    let output = run_git(&["config", "--null", "--list"])?;
    if !output.status.success() {
        return Err(format!(
            "cannot read Git comment prefix (Git exited with {})",
            output.status
        ));
    }
    parse_config_output(&output.stdout)
}

fn valid_repository_output(output: &[u8]) -> bool {
    let Some(path) = output.strip_suffix(b"\n") else {
        return false;
    };
    !path.is_empty() && !path.contains(&b'\0')
}

const BOUNDARY: &str = "Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).";
const NO_REPOSITORY: &str = "not a git repository (or any of the parent directories): .git";
const MOUNT_PREFIX: &str = "not a git repository (or any parent up to mount point ";

fn is_outside_repository(stderr: &str) -> bool {
    let stderr = parse_git_stderr(stderr);
    let diagnostic = stderr.diagnostics.join("\n");
    let Some(message) = diagnostic.strip_prefix("fatal: ") else {
        return false;
    };
    let outside = message == NO_REPOSITORY || is_boundary_message(message);
    outside && stderr.trace_errors.iter().all(|error| error == message)
}

fn is_boundary_message(message: &str) -> bool {
    message
        .strip_prefix(MOUNT_PREFIX)
        .and_then(|mount| mount.strip_suffix(&format!(")\n{BOUNDARY}")))
        .is_some_and(|mount| !mount.is_empty() && !mount.contains(['\n', '\r']))
}

#[derive(Default)]
struct GitStderr {
    diagnostics: Vec<String>,
    trace_errors: Vec<String>,
}

enum TraceLine {
    Info,
    BriefInfo { name: String, perf: bool },
    Error(String),
    Other,
}

// A Trace2 error can repeat the fatal diagnostic. It must not be discarded for
// config queries, and discovery only accepts it when it matches the real fatal.
fn parse_git_stderr(stderr: &str) -> GitStderr {
    let (brief, perf_brief) = brief_discovery_envelopes(stderr);
    let mut result = GitStderr::default();
    let mut normal_events = std::collections::HashSet::new();
    let mut perf_events = std::collections::HashSet::new();
    let mut lines = stderr.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        let Some(terminated) = line.strip_suffix('\n') else {
            result.diagnostics.push(line.to_owned());
            continue;
        };
        let line = terminated.strip_suffix('\r').unwrap_or(terminated);
        match classify_trace(line, brief, perf_brief) {
            TraceLine::Info => {}
            TraceLine::BriefInfo { name, perf } => {
                let events = if perf {
                    &mut perf_events
                } else {
                    &mut normal_events
                };
                if !events.insert(name) {
                    result.diagnostics.push(line.to_owned());
                }
            }
            TraceLine::Error(mut message) => {
                if message.starts_with(MOUNT_PREFIX)
                    && message.ends_with(')')
                    && lines
                        .peek()
                        .is_some_and(|next| next.trim_end_matches(['\r', '\n']) == BOUNDARY)
                {
                    lines.next();
                    message.push('\n');
                    message.push_str(BOUNDARY);
                }
                result.trace_errors.push(message);
            }
            TraceLine::Other => result.diagnostics.push(line.to_owned()),
        }
    }
    result
}

#[derive(Deserialize)]
struct Trace2Event {
    event: String,
    sid: String,
    thread: String,
    msg: Option<String>,
}

fn trace2_event(name: &str, message: &str) -> TraceLine {
    match name {
        "error" => TraceLine::Error(message.to_owned()),
        "version" | "start" | "exit" | "atexit" | "cmd_ancestry" | "cmd_name" | "cmd_path"
        | "cmd_mode" | "worktree" | "def_repo" | "def_param" | "alias" | "child_start"
        | "child_exit" | "exec" | "exec_result" | "thread_start" | "thread_exit"
        | "region_enter" | "region_leave" | "data" | "data_json" => TraceLine::Info,
        _ => TraceLine::Other,
    }
}

fn brief_trace2_event(name: &str, message: &str, perf: bool) -> TraceLine {
    match name {
        "error" => TraceLine::Error(message.to_owned()),
        "def_param" if valid_brief_parameter(message, perf) => TraceLine::Info,
        "data_json" => TraceLine::Info,
        "version" | "start" | "cmd_ancestry" | "cmd_name" | "exit" | "atexit" => {
            TraceLine::BriefInfo {
                name: name.to_owned(),
                perf,
            }
        }
        _ => TraceLine::Other,
    }
}

fn valid_brief_parameter(message: &str, perf: bool) -> bool {
    let parameter = if perf {
        message.split_once(':').map(|(key, _)| key)
    } else {
        message
            .strip_prefix("scope:")
            .and_then(|value| value.split_once(' '))
            .filter(|(scope, _)| {
                matches!(
                    *scope,
                    "system" | "global" | "local" | "worktree" | "command"
                )
            })
            .and_then(|(_, value)| value.split_once('='))
            .map(|(key, _)| key)
    };
    parameter.is_some_and(|key| valid_config_key(key.as_bytes()))
}

fn brief_discovery_envelopes(stderr: &str) -> (bool, bool) {
    let mut normal = 0;
    let mut perf = 0;
    for line in stderr.lines() {
        if let Some((name, message)) = line.split_once(' ') {
            normal = advance_discovery_envelope(normal, name, message);
        }
        if let Some((name, message)) = perf_event(line) {
            perf = advance_discovery_envelope(perf, name, message);
        }
    }
    (normal == 5, perf == 5)
}

fn advance_discovery_envelope(state: u8, name: &str, message: &str) -> u8 {
    match (state, name) {
        (0, "version") if !message.is_empty() => 1,
        (1, "start") if message.ends_with(" rev-parse --git-dir") => 2,
        (2, "cmd_name") if message.starts_with("rev-parse") => 3,
        (3, "exit") if message.ends_with("code:128") => 4,
        (4, "atexit") if message.ends_with("code:128") => 5,
        _ => state,
    }
}

fn perf_event(perf: &str) -> Option<(&str, &str)> {
    let fields: Vec<_> = perf.splitn(8, '|').map(str::trim).collect();
    if fields.len() != 8
        || !fields[0]
            .strip_prefix('d')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        || fields[1].is_empty()
    {
        return None;
    }
    Some((fields[2], fields[7]))
}

fn classify_trace(line: &str, brief: bool, perf_brief: bool) -> TraceLine {
    if line.starts_with('{') {
        let Ok(event) = serde_json::from_str::<Trace2Event>(line) else {
            return TraceLine::Other;
        };
        if event.sid.is_empty()
            || event.thread.is_empty()
            || (event.event == "error" && event.msg.is_none())
        {
            return TraceLine::Other;
        }
        return trace2_event(&event.event, event.msg.as_deref().unwrap_or_default());
    }

    let header = trace_header_payload(line);
    let payload = header.unwrap_or(line);
    if ["trace: ", "setup: ", "performance: "]
        .iter()
        .any(|prefix| payload.starts_with(prefix))
    {
        return TraceLine::Info;
    }
    let perf = if header.is_some() {
        payload.strip_prefix('|')
    } else if perf_brief {
        Some(payload)
    } else {
        None
    };
    if let Some((name, message)) = perf.and_then(perf_event) {
        return if header.is_some() {
            trace2_event(name, message)
        } else {
            brief_trace2_event(name, message, true)
        };
    }
    if header.is_none() && !brief {
        return TraceLine::Other;
    }
    let Some((name, message)) = payload.split_once(' ') else {
        return TraceLine::Other;
    };
    if header.is_some() {
        trace2_event(name, message)
    } else {
        brief_trace2_event(name, message, false)
    }
}

fn trace_header_payload(line: &str) -> Option<&str> {
    let (timestamp, rest) = line.split_once(' ')?;
    let time = timestamp.as_bytes();
    if time.len() != 15
        || time[2] != b':'
        || time[5] != b':'
        || time[8] != b'.'
        || time
            .iter()
            .enumerate()
            .any(|(i, b)| ![2, 5, 8].contains(&i) && !b.is_ascii_digit())
    {
        return None;
    }
    let (source, payload) = rest.trim_start().split_once(' ')?;
    let (file, number) = source.rsplit_once(':')?;
    if file.is_empty() || number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(payload.trim_start())
}

fn parse_config_output(output: &[u8]) -> Result<String, String> {
    let invalid = || "invalid output from Git configuration".to_owned();
    if output.is_empty() {
        return Ok("#".to_owned());
    }
    let records = output.strip_suffix(b"\0").ok_or_else(invalid)?;
    let mut selected = None;
    for record in records.split(|byte| *byte == b'\0') {
        let separator = record.iter().position(|byte| *byte == b'\n');
        let key = separator.map_or(record, |at| &record[..at]);
        if !valid_config_key(key) {
            return Err(invalid());
        }
        if key.eq_ignore_ascii_case(b"core.commentchar")
            || key.eq_ignore_ascii_case(b"core.commentstring")
        {
            selected = Some(separator.map(|at| &record[at + 1..]));
        }
    }
    let Some(value) = selected else {
        return Ok("#".to_owned());
    };
    let value = value.ok_or_else(|| "Git comment prefix must have a value".to_owned())?;
    let value =
        std::str::from_utf8(value).map_err(|_| "Git comment prefix must be UTF-8".to_owned())?;
    if value.is_empty() || value.contains(['\n', '\r']) {
        return Err("Git comment prefix must be nonempty and contain no newline".to_owned());
    }
    if value.eq_ignore_ascii_case("auto") {
        return Err("Git comment prefix 'auto' is not supported".to_owned());
    }
    Ok(value.to_owned())
}

fn valid_config_key(key: &[u8]) -> bool {
    let Some(last_dot) = key.iter().rposition(|byte| *byte == b'.') else {
        return false;
    };
    let section = &key[..last_dot];
    let variable = &key[last_dot + 1..];
    section
        .first()
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.'))
        && variable.first().is_some_and(u8::is_ascii_alphabetic)
        && variable
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_path_must_be_nonempty_nul_free_and_lf_terminated() {
        for valid in [&b".git\n"[..], &b"dir\nname\n"[..], &b"\n\n"[..]] {
            assert!(valid_repository_output(valid), "{valid:?}");
        }
        for invalid in [&b""[..], &b"\n"[..], &b".git"[..], &b"dir\0name\n"[..]] {
            assert!(!valid_repository_output(invalid), "{invalid:?}");
        }
    }

    #[test]
    fn discovery_only_accepts_complete_known_outside_repository_diagnostics() {
        let plain = "fatal: not a git repository (or any of the parent directories): .git";
        let boundary = "fatal: not a git repository (or any parent up to mount point /dev)\nStopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).";
        for diagnostic in [plain, boundary] {
            assert!(is_outside_repository(diagnostic));
            assert!(is_outside_repository(&format!(
                "12:34:56.123456 git.c:123 trace: running\n{diagnostic}\n"
            )));
            assert!(!is_outside_repository(&format!(
                "{diagnostic}\nerror: unrelated failure\n"
            )));
        }
        assert!(!is_outside_repository(
            "fatal: not a git repository (or any parent up to mount point /dev)"
        ));
        assert!(!is_outside_repository(
            "fatal: not a git repository (or any parent up to mount point )\nStopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set)."
        ));
        assert!(!is_outside_repository(
            "fatal: not a git repository (or any parent up to mount point /dev)\nerror: unrelated failure (\n)\nStopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set)."
        ));
    }

    #[test]
    fn trace_lines_are_ignored_but_unrelated_diagnostics_are_retained() {
        let trace = "12:34:56.123456 git.c:502 trace: built-in: git config\ntrace: bare output\n12:34:56.123456 trace.c:1 setup: git_dir: .git\n";
        assert!(parse_git_stderr(trace).diagnostics.is_empty());
        assert_eq!(
            parse_git_stderr(&format!("{trace}error: real failure\n")).diagnostics,
            vec!["error: real failure"]
        );
        assert_eq!(parse_git_stderr("\n").diagnostics, vec![""]);
        assert_eq!(
            parse_git_stderr("trace: incomplete").diagnostics,
            vec!["trace: incomplete"]
        );
        assert_eq!(
            parse_git_stderr("bogus timestamp git.c:502 trace: not Git output\n").diagnostics,
            vec!["bogus timestamp git.c:502 trace: not Git output"]
        );
    }

    #[test]
    fn brief_trace_requires_a_complete_envelope_and_keeps_unrelated_diagnostics() {
        let prefix = "version 2.54\nstart /usr/bin/git rev-parse --git-dir\ncmd_name rev-parse (rev-parse)\nerror not a git repository (or any of the parent directories): .git\nfatal: not a git repository (or any of the parent directories): .git\n";
        let suffix = "exit elapsed:0.01 code:128\natexit elapsed:0.01 code:128\n";
        assert!(is_outside_repository(&format!("{prefix}{suffix}")));
        assert!(!is_outside_repository(prefix));
        for extra in [
            "error: unrelated failure\n",
            "version unexpected failure\n",
            "worktree unexpected failure\n",
        ] {
            assert!(
                !is_outside_repository(&format!("{prefix}{extra}{suffix}")),
                "{extra}"
            );
        }
        assert!(!is_outside_repository(
            "version unrelated failure\nfatal: not a git repository (or any of the parent directories): .git\n"
        ));
    }

    #[test]
    fn brief_perf_data_json_events_do_not_hide_outside_repository_errors() {
        let no_repo = "not a git repository (or any of the parent directories): .git";
        let trace = format!(
            "d0 | main | version | | | | | 2.56.0.windows.4\n\
             d0 | main | start | | | | | git.exe rev-parse --git-dir\n\
             d0 | main | data_json | r0 | 0.025744 | 0.025744 | process | windows/ancestry:[...]\n\
             d0 | main | cmd_name | | | | | rev-parse (rev-parse)\n\
             d0 | main | error | | | | | {no_repo}\n\
             fatal: {no_repo}\n\
             d0 | main | data_json | r0 | 0.026283 | 0.026283 | process | windows/memory:{{...}}\n\
             d0 | main | exit | | 0.026305 | | | code:128\n\
             d0 | main | atexit | | 0.026315 | | | code:128\n"
        );

        assert!(is_outside_repository(&trace), "{trace}");
    }

    #[test]
    fn trace2_errors_are_never_discarded_or_used_as_fatal_diagnostics() {
        let no_repo = "not a git repository (or any of the parent directories): .git";
        let trace_error =
            format!(r#"{{"event":"error","sid":"test","thread":"main","msg":"{no_repo}"}}"#);
        assert!(!is_outside_repository(&format!("{trace_error}\n")));
        assert!(is_outside_repository(&format!(
            "{trace_error}\nfatal: {no_repo}\n"
        )));
        let different_error = r#"{"event":"error","sid":"test","thread":"main","msg":"unrelated"}"#;
        assert!(!is_outside_repository(&format!(
            "{different_error}\nfatal: {no_repo}\n"
        )));
        let trace = parse_git_stderr(&format!("{trace_error}\n"));
        assert_eq!(trace.trace_errors, vec![no_repo]);
        assert!(trace.diagnostics.is_empty());
        for malformed in [
            r#"{"event":"error","sid":"test","thread":"main"}"#,
            r#"{"event":"error","sid":"test","thread":"main","msg":"x""#,
            r#"{"event":"error","sid":"","thread":"main","msg":"x"}"#,
            r#"{"event":"error","sid":"test","thread":"main","msg":1}"#,
        ] {
            assert_eq!(
                parse_git_stderr(&format!("{malformed}\n")).diagnostics,
                vec![malformed]
            );
        }
    }

    #[test]
    fn trace2_metadata_cannot_spoof_repository_discovery() {
        let no_repo = "not a git repository (or any of the parent directories): .git";
        let fake = format!(
            r#"{{"event":"version","sid":"test","thread":"main","msg":"fatal: {no_repo}","detail":"\\\"event\\\":\\\"error\\\""}}"#
        );
        assert!(!is_outside_repository(&format!("{fake}\n")));
        assert!(is_outside_repository(&format!(
            "{fake}\nfatal: {no_repo}\n"
        )));
        let trace = format!("12:34:56.123456 usage.c:1 | d0 | main | error | | | | | {no_repo}\n");
        assert!(!is_outside_repository(&trace));
        assert!(is_outside_repository(&format!("{trace}fatal: {no_repo}\n")));
    }

    #[test]
    fn config_output_preserves_order_and_ignores_unrelated_binary_values() {
        assert_eq!(parse_config_output(b"").unwrap(), "#");
        assert_eq!(
            parse_config_output(b"other.key\n\xff\0other.valueless\0").unwrap(),
            "#"
        );
        assert_eq!(
            parse_config_output(
                b"core.commentchar\n#\0other.test\n\xff\0core.commentstring\n ! \0"
            )
            .unwrap(),
            " ! "
        );
        assert_eq!(
            parse_config_output(b"core.commentchar\0core.commentstring\n!!\0").unwrap(),
            "!!"
        );
        assert_eq!(
            parse_config_output(b"core.commentchar\n\xff\0core.commentstring\n!!\0").unwrap(),
            "!!"
        );
        assert_eq!(parse_config_output(b"http.\xff.url\n\xff\0").unwrap(), "#");
        assert_eq!(
            parse_config_output(b".section.key\nvalue\0..name\nvalue\0").unwrap(),
            "#"
        );
    }

    #[test]
    fn malformed_config_output_and_selected_values_are_rejected() {
        for output in [
            &b"core.commentchar\n#"[..],
            &b"badkey\nvalue\0"[..],
            &b".variable\nvalue\0"[..],
            &b"section.\nvalue\0"[..],
            &b"section.1invalid\nvalue\0"[..],
            &b"section.key\nvalue\0\0"[..],
            &b"core.commentchar\n#\0invalidkey\0"[..],
            &b"core.commentchar\n#\0section.key\nunterminated"[..],
            &b"core.commentchar\0"[..],
            &b"core.commentchar\n\0"[..],
            &b"core.commentchar\nfoo\nbar\0"[..],
            &b"core.commentchar\n\xff\0"[..],
            &b"core.commentchar\nAuto\0"[..],
            &b"core.commentchar\n#\0core.commentstring\0"[..],
        ] {
            assert!(parse_config_output(output).is_err(), "{output:?}");
        }
    }
}

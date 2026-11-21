mod cli_policy;
mod history;

use clap::Parser;
use commit_msg_lint::{config, lint, schema};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "commit-msg-lint",
    version,
    about = "Lint a message from standard input, a file, or selected Git commits."
)]
struct Arguments {
    #[arg(
        long,
        value_name = "PATH",
        help = "Read the message from a file instead of standard input."
    )]
    edit: Option<PathBuf>,
    #[arg(
        long,
        value_name = "REVISION-OR-RANGE",
        conflicts_with = "edit",
        help = "Lint messages stored in one commit or selected by an explicit Git range."
    )]
    commits: Option<String>,
    #[arg(
        long,
        value_name = "PATH",
        help = "Use the specified configuration file instead of discovering one."
    )]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Treat Lint Rule warnings as failures without changing their severity."
    )]
    strict: bool,
    #[arg(
        long,
        help = "Print agent-oriented usage guidance and exit. Cannot be combined with other options."
    )]
    help_for_agents: bool,
    #[arg(
        long,
        help = "Print the built-in default configuration as JSON and exit. Cannot be combined with other options."
    )]
    print_default_config: bool,
    #[arg(
        long,
        help = "Print the configuration JSON Schema and exit. Cannot be combined with other options."
    )]
    print_config_schema: bool,
}

fn discover_config() -> Result<Option<PathBuf>, String> {
    let output = cli_policy::git_command()
        .args(["rev-parse", "--show-toplevel"])
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("cannot discover Git repository: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Ok(None);
        }
        return Err(format!("cannot discover Git repository: {}", stderr.trim()));
    }
    let root = String::from_utf8(output.stdout)
        .map_err(|e| format!("Git repository path is not UTF-8: {e}"))?;
    let path = Path::new(root.trim_end_matches(['\r', '\n'])).join("commit-msg-lint.jsonc");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(path)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot inspect {}: {e}", path.display())),
    }
}

fn load_config(config_path: Option<PathBuf>) -> Result<config::Config, String> {
    let config_path = match config_path {
        Some(path) => Some(path),
        None => discover_config()?,
    };
    match config_path {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            config::Config::parse(&content)
                .map_err(|e| format!("invalid configuration {}: {e}", path.display()))
        }
        None => Ok(config::Config::default()),
    }
}

fn history_git_command() -> std::process::Command {
    let mut command = cli_policy::git_command();
    // Older Git versions ignore GIT_NO_LAZY_FETCH; deny transports as a fallback.
    command
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "");
    command
}

fn git_output(args: &[&str]) -> Result<std::process::Output, String> {
    history_git_command()
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("cannot run Git: {e}"))
}

fn git_error(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_owned()
}

fn resolve_commit(revision: &str) -> Result<String, String> {
    if revision.starts_with('-') {
        return Err("commit revision must not begin with '-'".to_owned());
    }
    let output = git_output(&["rev-parse", "--verify", "--end-of-options", revision])?;
    if !output.status.success() {
        return Err(format!(
            "cannot resolve commit revision {revision:?}: {}",
            git_error(&output)
        ));
    }
    let oid = parse_commit_id(&output.stdout)?;
    let expression = format!("{oid}^{{commit}}");
    let output = git_output(&["rev-parse", "--verify", "--end-of-options", &expression])?;
    if !output.status.success() {
        return Err(format!(
            "cannot resolve commit revision {revision:?}: {}",
            git_error(&output)
        ));
    }
    parse_commit_id(&output.stdout)
}

fn parse_commit_id(output: &[u8]) -> Result<String, String> {
    let oid = output
        .strip_suffix(b"\n")
        .filter(|oid| !oid.is_empty() && oid.iter().all(u8::is_ascii_hexdigit))
        .ok_or_else(|| "invalid commit ID from Git".to_owned())?;
    String::from_utf8(oid.to_vec()).map_err(|error| format!("invalid commit ID from Git: {error}"))
}

enum CommitSelection<'a> {
    Single,
    Range {
        left: &'a str,
        right: &'a str,
        symmetric: bool,
    },
}

fn parse_commit_selection(revision: &str) -> Result<CommitSelection<'_>, String> {
    let Some(separator) = revision.find("..") else {
        return Ok(CommitSelection::Single);
    };
    let left = &revision[..separator];
    let rest = &revision[separator + 2..];
    let (right, symmetric) = if let Some(right) = rest.strip_prefix('.') {
        (right, true)
    } else {
        (rest, false)
    };
    if left.is_empty() || right.is_empty() || right.starts_with('.') {
        return Err(
            "commit range must have two explicit endpoints separated by '..' or '...'".to_owned(),
        );
    }
    Ok(CommitSelection::Range {
        left,
        right,
        symmetric,
    })
}

fn selected_commits(revision: &str) -> Result<(String, bool, usize), String> {
    let shallow = git_output(&["rev-parse", "--is-shallow-repository"])?;
    if !shallow.status.success() {
        return Err(format!(
            "cannot determine whether repository is shallow: {}",
            git_error(&shallow)
        ));
    }
    match shallow.stdout.as_slice() {
        b"true\n" => {
            return Err(
                "shallow repository; fetch the required history separately before linting"
                    .to_owned(),
            );
        }
        b"false\n" => {}
        _ => return Err("invalid shallow repository status from Git".to_owned()),
    }
    let (left, right, symmetric) = match resolve_commit(revision) {
        Ok(oid) => {
            let oid_len = oid.len();
            return Ok((oid, true, oid_len));
        }
        Err(single_error) => match parse_commit_selection(revision)? {
            CommitSelection::Single => return Err(single_error),
            CommitSelection::Range {
                left,
                right,
                symmetric,
            } => (left, right, symmetric),
        },
    };
    let left = resolve_commit(left)?;
    let right = resolve_commit(right)?;
    let separator = if symmetric { "..." } else { ".." };
    if left.len() != right.len() {
        return Err("inconsistent commit IDs from Git".to_owned());
    }
    Ok((format!("{left}{separator}{right}"), false, left.len()))
}

fn write_stdout(content: &str, description: &str) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(content.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("cannot write {description} to standard output: {error}"))
}

fn run() -> Result<u8, String> {
    let raw_args: Vec<_> = std::env::args_os().skip(1).collect();
    for option in [
        "--help-for-agents",
        "--print-default-config",
        "--print-config-schema",
    ] {
        if raw_args.len() != 1 && raw_args.iter().any(|argument| argument == option) {
            return Err(format!("{option} cannot be combined with other arguments"));
        }
    }

    let args = Arguments::parse();
    if args.print_config_schema {
        let schema = schema::config_schema_json()
            .map_err(|error| format!("cannot serialize configuration schema: {error}"))?;
        write_stdout(&schema, "configuration schema")?;
        return Ok(0);
    }
    if args.print_default_config {
        let defaults = config::Config::default_json()
            .map_err(|error| format!("cannot serialize default configuration: {error}"))?;
        write_stdout(&defaults, "default configuration")?;
        return Ok(0);
    }
    if args.help_for_agents {
        write_stdout(include_str!("../docs/agent-usage.md"), "agent usage")?;
        return Ok(0);
    }

    if let Some(revision) = args.commits.as_deref() {
        let config = load_config(args.config)?;
        return history::lint_commits(revision, &config, args.strict);
    }

    let policy = cli_policy::FilterPolicy::from_environment()?;
    let prefix = policy.comment_prefix()?;
    let config = load_config(args.config)?;
    let (source, input) = match args.edit {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            (path.to_string_lossy().into_owned(), content)
        }
        None => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|e| format!("cannot read standard input: {e}"))?;
            ("<stdin>".to_string(), input)
        }
    };
    let diagnostics = lint::lint_filtered(
        &input,
        &config,
        &prefix,
        policy.strip_comments,
        policy.cut_scissors,
    );
    let mut failed = false;
    for diagnostic in diagnostics {
        eprintln!(
            "{}:{}: {}[{}] {}",
            source, diagnostic.line, diagnostic.severity, diagnostic.rule, diagnostic.message
        );
        failed |= diagnostic.severity == config::Severity::Error || args.strict;
    }
    Ok(u8::from(failed))
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "commit-msg-lint: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn only_history_git_commands_override_fetch_environment() {
        let ordinary = cli_policy::git_command();
        for (name, value) in [("GIT_NO_LAZY_FETCH", "1"), ("GIT_ALLOW_PROTOCOL", "")] {
            assert!(!ordinary.get_envs().any(|(key, _)| key == OsStr::new(name)));
            assert!(
                history_git_command()
                    .get_envs()
                    .any(|(key, actual)| key == OsStr::new(name)
                        && actual == Some(OsStr::new(value)))
            );
        }
    }
}

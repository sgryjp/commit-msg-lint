mod cli_policy;

use clap::Parser;
use commit_msg_lint::{config, lint, schema};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "commit-msg-lint",
    version,
    about = "Lint a message from standard input, a file, or one Git commit."
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
        value_name = "REVISION",
        conflicts_with = "edit",
        help = "Lint the message stored in one Git commit."
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
        help = "Treat warnings as failures for the exit code without changing their severity."
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

fn git_output(args: &[&str]) -> Result<std::process::Output, String> {
    cli_policy::git_command()
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
    let expression = format!("{revision}^{{commit}}");
    let output = git_output(&["rev-parse", "--verify", "--end-of-options", &expression])?;
    if !output.status.success() {
        return Err(format!(
            "cannot resolve commit revision {revision:?}: {}",
            git_error(&output)
        ));
    }
    let oid = output
        .stdout
        .strip_suffix(b"\n")
        .filter(|oid| !oid.is_empty() && oid.iter().all(u8::is_ascii_hexdigit))
        .ok_or_else(|| "invalid commit ID from Git".to_owned())?;
    String::from_utf8(oid.to_vec()).map_err(|e| format!("invalid commit ID from Git: {e}"))
}

fn commit_message(oid: &str) -> Result<String, String> {
    let output = git_output(&["cat-file", "commit", oid])?;
    if !output.status.success() {
        return Err(format!("cannot read commit {oid}: {}", git_error(&output)));
    }
    let separator = output
        .stdout
        .windows(2)
        .position(|window| window == b"\n\n")
        .ok_or_else(|| format!("cannot read commit {oid}: malformed commit object"))?;
    let message = std::str::from_utf8(&output.stdout[separator + 2..])
        .map_err(|e| format!("commit {oid} message is not valid UTF-8: {e}"))?;
    Ok(message.to_owned())
}

fn lint_commit(revision: &str, config: &config::Config, strict: bool) -> Result<u8, String> {
    let oid = resolve_commit(revision)?;
    let message = commit_message(&oid)?;
    let mut failed = false;
    for diagnostic in lint::lint(&message, config) {
        eprintln!(
            "{}:{}: {}[{}] {}",
            oid, diagnostic.line, diagnostic.severity, diagnostic.rule, diagnostic.message
        );
        failed |= diagnostic.severity == config::Severity::Error || strict;
    }
    Ok(u8::from(failed))
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
        return lint_commit(revision, &config, args.strict);
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
            eprintln!("commit-msg-lint: {error}");
            ExitCode::from(2)
        }
    }
}

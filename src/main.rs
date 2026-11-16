mod cli_policy;

use clap::Parser;
use commit_msg_lint::{config, lint};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "commit-msg-lint",
    version,
    about = "Lint a commit message using the configured Rules."
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

fn write_stdout(content: &str, description: &str) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(content.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("cannot write {description} to standard output: {error}"))
}

fn run() -> Result<u8, String> {
    let raw_args: Vec<_> = std::env::args_os().skip(1).collect();
    for option in ["--help-for-agents", "--print-default-config"] {
        if raw_args.len() != 1 && raw_args.iter().any(|argument| argument == option) {
            return Err(format!("{option} cannot be combined with other arguments"));
        }
    }

    let args = Arguments::parse();
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

    let policy = cli_policy::FilterPolicy::from_environment()?;
    let prefix = policy.comment_prefix()?;
    let config_path = match args.config {
        Some(path) => Some(path),
        None => discover_config()?,
    };
    let config = match config_path {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            config::Config::parse(&content)
                .map_err(|e| format!("invalid configuration {}: {e}", path.display()))?
        }
        None => config::Config::default(),
    };
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

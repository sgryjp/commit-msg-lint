mod cli_policy;

use clap::Parser;
use commit_msg_lint::{config, lint, schema};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};

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

enum CommitSelection<'a> {
    Single(&'a str),
    Range {
        left: &'a str,
        right: &'a str,
        symmetric: bool,
    },
}

fn parse_commit_selection(revision: &str) -> Result<CommitSelection<'_>, String> {
    let Some(separator) = revision.find("..") else {
        return Ok(CommitSelection::Single(revision));
    };
    let left = &revision[..separator];
    let rest = &revision[separator + 2..];
    let (right, symmetric) = if let Some(right) = rest.strip_prefix('.') {
        (right, true)
    } else {
        (rest, false)
    };
    if left.is_empty() || right.is_empty() || right.starts_with('.') || right.contains("..") {
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

fn selected_commits(revision: &str) -> Result<Vec<String>, String> {
    match parse_commit_selection(revision)? {
        CommitSelection::Single(revision) => Ok(vec![resolve_commit(revision)?]),
        CommitSelection::Range {
            left,
            right,
            symmetric,
        } => {
            let left = resolve_commit(left)?;
            let right = resolve_commit(right)?;
            let shallow = git_output(&["rev-parse", "--is-shallow-repository"])?;
            if !shallow.status.success() {
                return Err(format!(
                    "cannot determine whether repository is shallow: {}",
                    git_error(&shallow)
                ));
            }
            match shallow.stdout.as_slice() {
                b"true\n" => eprintln!(
                    "warning: shallow repository; commit range selection may be incomplete"
                ),
                b"false\n" => {}
                _ => return Err("invalid shallow repository status from Git".to_owned()),
            }
            let separator = if symmetric { "..." } else { ".." };
            let range = format!("{left}{separator}{right}");
            let output = git_output(&["rev-list", "--topo-order", "--reverse", &range])?;
            if !output.status.success() {
                return Err(format!(
                    "cannot select commits in range {revision:?}: {}",
                    git_error(&output)
                ));
            }
            let Some(ids) = output.stdout.strip_suffix(b"\n") else {
                if output.stdout.is_empty() {
                    return Ok(Vec::new());
                }
                return Err("invalid commit IDs from Git".to_owned());
            };
            ids.split(|byte| *byte == b'\n')
                .map(|id| {
                    if id.is_empty() || !id.iter().all(u8::is_ascii_hexdigit) {
                        return Err("invalid commit ID from Git".to_owned());
                    }
                    String::from_utf8(id.to_vec())
                        .map_err(|error| format!("invalid commit ID from Git: {error}"))
                })
                .collect()
        }
    }
}

struct CommitBatch {
    output: std::process::Output,
    input_error: Option<String>,
}

fn read_commit_batch(commits: &[String]) -> Result<CommitBatch, String> {
    let mut child = cli_policy::git_command()
        .args(["cat-file", "--batch"])
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run Git: {error}"))?;
    let mut stdin = child.stdin.take().expect("piped Git stdin");
    let requests = format!("{}\n", commits.join("\n"));
    let writer = std::thread::spawn(move || stdin.write_all(requests.as_bytes()));
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot read Git output: {error}"))?;
    let input_error = writer
        .join()
        .map_err(|_| "cannot write commit IDs to Git".to_owned())?
        .err()
        .map(|error| format!("cannot write commit IDs to Git: {error}"));
    Ok(CommitBatch {
        output,
        input_error,
    })
}

fn read_batch_message<'a>(
    output: &'a [u8],
    cursor: &mut usize,
    oid: &str,
) -> Result<&'a [u8], String> {
    let line_end = output[*cursor..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|length| *cursor + length)
        .ok_or_else(|| format!("cannot read commit {oid}: malformed Git batch output"))?;
    let header = std::str::from_utf8(&output[*cursor..line_end])
        .map_err(|error| format!("cannot read commit {oid}: invalid Git batch output: {error}"))?;
    let fields: Vec<_> = header.split_ascii_whitespace().collect();
    if fields == [oid, "missing"] {
        return Err(format!("cannot read commit {oid}: object is unavailable"));
    }
    if fields.len() != 3 || fields[0] != oid || fields[1] != "commit" {
        return Err(format!(
            "cannot read commit {oid}: malformed Git batch output"
        ));
    }
    let size = fields[2]
        .parse::<usize>()
        .map_err(|_| format!("cannot read commit {oid}: invalid object size from Git"))?;
    let start = line_end + 1;
    let end = start
        .checked_add(size)
        .filter(|end| *end < output.len() && output[*end] == b'\n')
        .ok_or_else(|| format!("cannot read commit {oid}: truncated Git batch output"))?;
    let object = &output[start..end];
    *cursor = end + 1;
    let separator = object
        .windows(2)
        .position(|window| window == b"\n\n")
        .ok_or_else(|| format!("cannot read commit {oid}: malformed commit object"))?;
    Ok(&object[separator + 2..])
}

fn lint_commits(revision: &str, config: &config::Config, strict: bool) -> Result<u8, String> {
    let commits = selected_commits(revision)?;
    if commits.is_empty() {
        return Ok(0);
    }
    let batch = read_commit_batch(&commits)?;
    let mut cursor = 0;
    let mut failed = false;
    for oid in &commits {
        let bytes = read_batch_message(&batch.output.stdout, &mut cursor, oid)?;
        let message = std::str::from_utf8(bytes)
            .map_err(|error| format!("commit {oid} message is not valid UTF-8: {error}"))?;
        for diagnostic in lint::lint(message, config) {
            eprintln!(
                "{}:{}: {}[{}] {}",
                oid, diagnostic.line, diagnostic.severity, diagnostic.rule, diagnostic.message
            );
            failed |= diagnostic.severity == config::Severity::Error || strict;
        }
    }
    if cursor != batch.output.stdout.len() {
        return Err("unexpected extra output from Git while reading commits".to_owned());
    }
    if let Some(error) = batch.input_error {
        return Err(error);
    }
    if !batch.output.status.success() {
        return Err(format!(
            "cannot read commits from Git: {}",
            git_error(&batch.output)
        ));
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
        return lint_commits(revision, &config, args.strict);
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

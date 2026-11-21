mod cli_policy;

use clap::Parser;
use commit_msg_lint::{config, lint, schema};
use std::io::{self, BufRead, Read, Write};
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

fn selected_commits(revision: &str) -> Result<Vec<String>, String> {
    let (left, right, symmetric) = match resolve_commit(revision) {
        Ok(oid) => return Ok(vec![oid]),
        Err(single_error) => match parse_commit_selection(revision) {
            Ok(CommitSelection::Single) => return Err(single_error),
            Ok(CommitSelection::Range {
                left,
                right,
                symmetric,
            }) => (left, right, symmetric),
            Err(range_error) => return Err(range_error),
        },
    };
    let shallow = git_output(&["rev-parse", "--is-shallow-repository"])?;
    if !shallow.status.success() {
        return Err(format!(
            "cannot determine whether repository is shallow: {}",
            git_error(&shallow)
        ));
    }
    match shallow.stdout.as_slice() {
        b"true\n" => {
            eprintln!("warning: shallow repository; commit range selection may be incomplete")
        }
        b"false\n" => {}
        _ => return Err("invalid shallow repository status from Git".to_owned()),
    }
    let left = resolve_commit(left)?;
    let right = resolve_commit(right)?;
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

fn read_batch_object(reader: &mut impl BufRead, oid: &str) -> Result<Vec<u8>, String> {
    let mut header = Vec::new();
    reader
        .read_until(b'\n', &mut header)
        .map_err(|error| format!("cannot read commit {oid}: {error}"))?;
    let header = header
        .strip_suffix(b"\n")
        .ok_or_else(|| format!("cannot read commit {oid}: malformed Git batch output"))?;
    let header = std::str::from_utf8(header)
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
    let mut remaining = fields[2]
        .parse::<usize>()
        .map_err(|_| format!("cannot read commit {oid}: invalid object size from Git"))?;
    let mut object = Vec::new();
    let mut chunk = [0; 8192];
    while remaining > 0 {
        let length = remaining.min(chunk.len());
        reader.read_exact(&mut chunk[..length]).map_err(|error| {
            format!("cannot read commit {oid}: truncated Git batch output: {error}")
        })?;
        object
            .try_reserve(length)
            .map_err(|error| format!("cannot read commit {oid}: object is too large: {error}"))?;
        object.extend_from_slice(&chunk[..length]);
        remaining -= length;
    }
    let mut separator = [0];
    reader.read_exact(&mut separator).map_err(|error| {
        format!("cannot read commit {oid}: truncated Git batch output: {error}")
    })?;
    if separator != [b'\n'] {
        return Err(format!(
            "cannot read commit {oid}: malformed Git batch output"
        ));
    }
    Ok(object)
}

fn read_batch_message<'a>(object: &'a [u8], oid: &str) -> Result<&'a [u8], String> {
    let separator = object
        .windows(2)
        .position(|window| window == b"\n\n")
        .ok_or_else(|| format!("cannot read commit {oid}: malformed commit object"))?;
    Ok(&object[separator + 2..])
}

struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn lint_commits(revision: &str, config: &config::Config, strict: bool) -> Result<u8, String> {
    let commits = selected_commits(revision)?;
    if commits.is_empty() {
        return Ok(0);
    }

    let mut child = cli_policy::git_command()
        .args(["cat-file", "--batch"])
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run Git: {error}"))?;
    let stdin = child.stdin.take().expect("piped Git stdin");
    let mut stdout = io::BufReader::new(child.stdout.take().expect("piped Git stdout"));
    let stderr = child.stderr.take().expect("piped Git stderr");

    std::thread::scope(|scope| {
        let mut child = ChildGuard(child);
        let commit_ids = &commits;
        let writer = scope.spawn(move || {
            let mut stdin = stdin;
            for oid in commit_ids {
                writeln!(stdin, "{oid}")
                    .map_err(|error| format!("cannot write commit IDs to Git: {error}"))?;
            }
            Ok::<(), String>(())
        });
        let stderr_reader = scope.spawn(move || {
            let mut stderr = stderr;
            let mut output = Vec::new();
            stderr
                .read_to_end(&mut output)
                .map(|_| output)
                .map_err(|error| format!("cannot read Git error output: {error}"))
        });

        let processing = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut diagnostic_output = io::stderr().lock();
            let mut failed = false;
            for oid in &commits {
                let object = read_batch_object(&mut stdout, oid)?;
                let bytes = read_batch_message(&object, oid)?;
                let message = std::str::from_utf8(bytes)
                    .map_err(|error| format!("commit {oid} message is not valid UTF-8: {error}"))?;
                for diagnostic in lint::lint(message, config) {
                    writeln!(
                        diagnostic_output,
                        "{}:{}: {}[{}] {}",
                        oid,
                        diagnostic.line,
                        diagnostic.severity,
                        diagnostic.rule,
                        diagnostic.message
                    )
                    .map_err(|error| {
                        format!("cannot write diagnostic to standard error: {error}")
                    })?;
                    failed |= diagnostic.severity == config::Severity::Error || strict;
                }
            }
            let mut extra = [0];
            if stdout
                .read(&mut extra)
                .map_err(|error| format!("cannot read Git output: {error}"))?
                != 0
            {
                return Err("unexpected extra output from Git while reading commits".to_owned());
            }
            Ok(failed)
        }))
        .unwrap_or_else(|_| Err("commit processing panicked".to_owned()));

        if processing.is_err() {
            let _ = child.0.kill();
        }
        let status = child
            .0
            .wait()
            .map_err(|error| format!("cannot wait for Git: {error}"));
        let input_result = writer
            .join()
            .map_err(|_| "cannot write commit IDs to Git".to_owned())
            .and_then(|result| result);
        let stderr_result = stderr_reader
            .join()
            .map_err(|_| "cannot read Git error output".to_owned())?;

        let failed = processing?;
        let status = status?;
        input_result?;
        let stderr = stderr_result?;
        if !status.success() {
            return Err(format!(
                "cannot read commits from Git: {}",
                String::from_utf8_lossy(&stderr).trim()
            ));
        }
        Ok(u8::from(failed))
    })
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
            let _ = writeln!(io::stderr().lock(), "commit-msg-lint: {error}");
            ExitCode::from(2)
        }
    }
}

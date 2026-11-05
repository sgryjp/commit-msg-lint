mod config;
mod lint;

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

struct Arguments {
    edit: Option<PathBuf>,
    config: Option<PathBuf>,
    strict: bool,
}

fn arguments() -> Result<Option<Arguments>, String> {
    let mut args = std::env::args_os().skip(1);
    let mut result = Arguments {
        edit: None,
        config: None,
        strict: false,
    };
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!("Usage: commit-msg-lint [--edit <path>] [--config <path>] [--strict]");
                return Ok(None);
            }
            Some("--version" | "-V") => {
                println!("commit-msg-lint {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            Some("--strict") if !result.strict => result.strict = true,
            Some("--edit") if result.edit.is_none() => {
                result.edit = Some(path_argument("--edit", args.next())?);
            }
            Some("--config") if result.config.is_none() => {
                result.config = Some(path_argument("--config", args.next())?);
            }
            _ => {
                return Err(format!(
                    "unknown or repeated argument: {}",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(Some(result))
}

fn path_argument(option: &str, value: Option<OsString>) -> Result<PathBuf, String> {
    match value {
        Some(value) if !value.is_empty() && !value.to_string_lossy().starts_with('-') => {
            Ok(value.into())
        }
        _ => Err(format!("{option} requires a path")),
    }
}

fn discover_config() -> Result<Option<PathBuf>, String> {
    let output = Command::new("git")
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
    let path = Path::new(root.trim_end_matches(['\r', '\n'])).join("commit-msg-lint.toml");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(path)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot inspect {}: {e}", path.display())),
    }
}

fn run() -> Result<u8, String> {
    let Some(args) = arguments()? else {
        return Ok(0);
    };
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
    let diagnostics = lint::lint(&input, &config);
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

use crate::cli_policy;
use commit_msg_lint::{config, lint};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::process::Stdio;

pub(crate) fn lint_commits(
    revision: &str,
    config: &config::Config,
    strict: bool,
) -> Result<u8, String> {
    let selection = selected_commits(revision)?;
    let file = retrieve_git_log(&selection)?;
    let mut records = LogReader::new(file, &selection)?;
    let mut diagnostic_output = io::stderr().lock();
    let mut failed = false;
    while let Some(record) = records.next_record()? {
        for diagnostic in lint::lint(&record.message, config) {
            writeln!(
                diagnostic_output,
                "{}:{}: {}[{}] {}",
                record.oid,
                diagnostic.line,
                diagnostic.severity,
                diagnostic.rule,
                diagnostic.message
            )
            .map_err(|error| format!("cannot write diagnostic to standard error: {error}"))?;
            failed |= diagnostic.severity == config::Severity::Error || strict;
        }
    }
    Ok(u8::from(failed))
}

fn retrieve_git_log(selection: &Selection) -> Result<File, String> {
    let mut file =
        tempfile::tempfile().map_err(|error| format!("cannot create temporary file: {error}"))?;
    let stdout = file
        .try_clone()
        .map_err(|error| format!("cannot open temporary file for Git: {error}"))?;
    // Git tracing can produce substantial stderr; spooling avoids pipe deadlocks and buffering it all.
    let mut errors = tempfile::tempfile()
        .map_err(|error| format!("cannot create temporary file for Git errors: {error}"))?;
    let stderr = errors
        .try_clone()
        .map_err(|error| format!("cannot open temporary file for Git errors: {error}"))?;
    let mut command = git_command();
    command.args([
        "-c",
        "log.showSignature=false",
        "-c",
        "log.decorate=no",
        "log",
        "--no-decorate",
        "--no-notes",
        "--no-show-signature",
        "--no-color",
        "--no-patch",
        "--encoding=UTF-8",
        "--log-size",
        "--format=tformat:%H%x00%B",
        "--topo-order",
        "--reverse",
    ]);
    if matches!(selection, Selection::Single(_)) {
        command.arg("--no-walk");
    }
    let status = command
        .arg(selection.argument())
        .arg("--")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .status()
        .map_err(|error| format!("cannot run Git: {error}"))?;
    if !status.success() {
        let error_len = errors
            .metadata()
            .map_err(|error| format!("cannot inspect Git error output: {error}"))?
            .len();
        errors
            .seek(SeekFrom::Start(error_len.saturating_sub(64 * 1024)))
            .map_err(|error| format!("cannot seek Git error output: {error}"))?;
        let mut stderr = Vec::new();
        errors
            .take(64 * 1024)
            .read_to_end(&mut stderr)
            .map_err(|error| format!("cannot read Git error output: {error}"))?;
        return Err(format!(
            "cannot read commits from Git: {}",
            String::from_utf8_lossy(&stderr).trim()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot rewind Git output: {error}"))?;
    Ok(file)
}

struct LogRecord {
    oid: String,
    message: String,
}

struct LogReader<'a> {
    reader: BufReader<File>,
    file_len: u64,
    selection: &'a Selection,
    records: usize,
}

impl<'a> LogReader<'a> {
    fn new(file: File, selection: &'a Selection) -> Result<Self, String> {
        let file_len = file
            .metadata()
            .map_err(|error| format!("cannot inspect Git output: {error}"))?
            .len();
        Ok(Self {
            reader: BufReader::new(file),
            file_len,
            selection,
            records: 0,
        })
    }

    fn next_record(&mut self) -> Result<Option<LogRecord>, String> {
        let reader = &mut self.reader;
        let oid_len = self.selection.oid_len();
        if reader
            .fill_buf()
            .map_err(|error| format!("cannot read Git output: {error}"))?
            .is_empty()
        {
            if matches!(self.selection, Selection::Single(_)) && self.records != 1 {
                return Err("missing commit from Git log".to_owned());
            }
            return Ok(None);
        }
        let mut header = Vec::new();
        (&mut *reader)
            .take(64)
            .read_until(b'\n', &mut header)
            .map_err(|error| format!("cannot read Git output: {error}"))?;
        let size = header
            .strip_prefix(b"log size ")
            .and_then(|value| value.strip_suffix(b"\n"))
            .filter(|value| !value.is_empty() && value.iter().all(u8::is_ascii_digit))
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| "malformed Git log size header".to_owned())?;
        // Git counts the ID and NUL in the payload size; tformat's newline is outside it.
        let position = reader
            .stream_position()
            .map_err(|error| format!("cannot read Git output position: {error}"))?;
        if size < (oid_len as u64).saturating_add(1)
            || (oid_len as u64).saturating_add(1) > self.file_len.saturating_sub(position)
        {
            return Err("truncated or invalid Git log frame".to_owned());
        }
        let mut oid_bytes = vec![0; oid_len];
        reader
            .read_exact(&mut oid_bytes)
            .map_err(|error| format!("cannot read Git commit ID: {error}"))?;
        if !oid_bytes.iter().all(u8::is_ascii_hexdigit) {
            return Err("invalid commit ID from Git log".to_owned());
        }
        let oid = std::str::from_utf8(&oid_bytes)
            .map_err(|_| "invalid commit ID from Git log".to_owned())?;
        if let Selection::Single(expected) = self.selection
            && (self.records != 0 || oid != expected)
        {
            return Err("unexpected commit ID from Git log".to_owned());
        }
        let mut separator = [0];
        reader
            .read_exact(&mut separator)
            .map_err(|error| format!("cannot read commit {oid}: {error}"))?;
        if separator != [0] {
            return Err(format!("malformed Git log frame for commit {oid}"));
        }
        if size
            .checked_add(1)
            .is_none_or(|length| length > self.file_len.saturating_sub(position))
        {
            return Err(format!("truncated Git log frame for commit {oid}"));
        }
        let mut remaining = size - oid_len as u64 - 1;
        let mut bytes = Vec::new();
        let mut chunk = [0; 8192];
        while remaining > 0 {
            let length = remaining.min(chunk.len() as u64) as usize;
            reader
                .read_exact(&mut chunk[..length])
                .map_err(|error| format!("cannot read commit {oid}: {error}"))?;
            bytes
                .try_reserve(length)
                .map_err(|error| format!("commit {oid} message is too large: {error}"))?;
            bytes.extend_from_slice(&chunk[..length]);
            remaining -= length as u64;
        }
        reader
            .read_exact(&mut separator)
            .map_err(|error| format!("cannot read commit {oid}: {error}"))?;
        if separator != *b"\n" {
            return Err(format!("malformed Git log terminator for commit {oid}"));
        }
        let message = String::from_utf8(bytes).map_err(|error| {
            format!(
                "commit {oid} message is not valid UTF-8: {}",
                error.utf8_error()
            )
        })?;
        self.records += 1;
        Ok(Some(LogRecord {
            oid: oid.to_owned(),
            message,
        }))
    }
}

fn git_command() -> std::process::Command {
    let mut command = cli_policy::git_command();
    // Older Git versions ignore GIT_NO_LAZY_FETCH; deny transports as a fallback.
    command
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "");
    command
}

fn git_output(args: &[&str]) -> Result<std::process::Output, String> {
    git_command()
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

enum Selection {
    Single(String),
    Range {
        left: String,
        right: String,
        symmetric: bool,
    },
}

impl Selection {
    fn argument(&self) -> String {
        match self {
            Self::Single(oid) => oid.clone(),
            Self::Range {
                left,
                right,
                symmetric,
            } => {
                let separator = if *symmetric { "..." } else { ".." };
                format!("{left}{separator}{right}")
            }
        }
    }

    fn oid_len(&self) -> usize {
        match self {
            Self::Single(oid) => oid.len(),
            Self::Range { left, .. } => left.len(),
        }
    }
}

fn resolve_range(revision: &str, single_error: String) -> Result<Selection, String> {
    let mut selected = None;
    let mut error = None;
    let mut found_separator = false;
    // Dots can occur in either endpoint's search expression. Accept only one
    // split whose two endpoints independently resolve to commits.
    for (index, _) in revision
        .match_indices('.')
        .filter(|(index, _)| revision[*index..].starts_with(".."))
    {
        found_separator = true;
        let left = &revision[..index];
        let rest = &revision[index + 2..];
        let (right, symmetric) = match rest.strip_prefix('.') {
            Some(right) => (right, true),
            None => (rest, false),
        };
        if left.is_empty() || right.is_empty() || right.starts_with('.') {
            continue;
        }
        let left = match resolve_commit(left) {
            Ok(oid) => oid,
            Err(candidate_error) => {
                if error.is_none() {
                    error = Some(candidate_error);
                }
                continue;
            }
        };
        let right = match resolve_commit(right) {
            Ok(oid) => oid,
            Err(candidate_error) => {
                error = Some(candidate_error);
                continue;
            }
        };
        if left.len() != right.len() {
            return Err("inconsistent commit IDs from Git".to_owned());
        }
        if selected.is_some() {
            return Err(
                "ambiguous commit range; use unambiguous revisions or commit IDs".to_owned(),
            );
        }
        selected = Some(Selection::Range {
            left,
            right,
            symmetric,
        });
    }
    selected.ok_or_else(|| {
        if !found_separator {
            single_error
        } else {
            error.unwrap_or_else(|| {
                "commit range must have two explicit endpoints separated by '..' or '...'"
                    .to_owned()
            })
        }
    })
}

fn selected_commits(revision: &str) -> Result<Selection, String> {
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
    match resolve_commit(revision) {
        Ok(oid) => Ok(Selection::Single(oid)),
        Err(single_error) => resolve_range(revision, single_error),
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
                git_command()
                    .get_envs()
                    .any(|(key, actual)| key == OsStr::new(name)
                        && actual == Some(OsStr::new(value)))
            );
        }
    }
}

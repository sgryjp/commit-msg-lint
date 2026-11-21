use crate::{history_git_command, selected_commits};
use commit_msg_lint::{config, lint};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::process::Stdio;

pub(crate) fn lint_commits(
    revision: &str,
    config: &config::Config,
    strict: bool,
) -> Result<u8, String> {
    let (selection, single, oid_len) = selected_commits(revision)?;
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
    let mut command = history_git_command();
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
    if single {
        command.arg("--no-walk");
    }
    let status = command
        .arg(&selection)
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
    let file_len = file
        .metadata()
        .map_err(|error| format!("cannot inspect Git output: {error}"))?
        .len();
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot rewind Git output: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut diagnostic_output = io::stderr().lock();
    let mut failed = false;
    let mut records = 0usize;
    loop {
        if reader
            .fill_buf()
            .map_err(|error| format!("cannot read Git output: {error}"))?
            .is_empty()
        {
            break;
        }
        let mut header = Vec::new();
        (&mut reader)
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
            || (oid_len as u64).saturating_add(1) > file_len.saturating_sub(position)
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
        if single && (records != 0 || oid != selection) {
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
            .is_none_or(|length| length > file_len.saturating_sub(position))
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
        let message = std::str::from_utf8(&bytes)
            .map_err(|error| format!("commit {oid} message is not valid UTF-8: {error}"))?;
        for diagnostic in lint::lint(message, config) {
            writeln!(
                diagnostic_output,
                "{}:{}: {}[{}] {}",
                oid, diagnostic.line, diagnostic.severity, diagnostic.rule, diagnostic.message
            )
            .map_err(|error| format!("cannot write diagnostic to standard error: {error}"))?;
            failed |= diagnostic.severity == config::Severity::Error || strict;
        }
        records += 1;
    }
    if single && records != 1 {
        return Err("missing commit from Git log".to_owned());
    }
    Ok(u8::from(failed))
}

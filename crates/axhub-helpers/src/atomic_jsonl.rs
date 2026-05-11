// Phase 26 PR 26.1a — shared atomic NDJSON append helper.
//
// telemetry.rs (phase markers) and audit.rs (routing audit) historically each
// owned their own `O_APPEND + writeln!` loop with subtle permission drift.
// This module hosts a single canonical `append_line` plus matching read +
// rotation helpers so the upcoming `event_log.rs` (PR 26.1b) and any other
// future NDJSON consumer share one well-tested implementation.
//
// Invariants enforced:
//   - parent directories are created when missing (best-effort)
//   - Unix: file is opened/created with mode 0o600 AND existing files are
//     re-enforced to 0o600 on every append (covers umask drift)
//   - POSIX guarantee: O_APPEND writes ≤ PIPE_BUF (≥ 4 KiB) are atomic
//     across concurrent writers — the same contract telemetry + audit have
//     always relied on
//   - corrupt-line skip + missing-file = empty Vec for read_lines, so
//     downstream callers never have to bring their own NotFound handling
//   - rotate_old is fail-soft on a per-entry basis (single bad file does
//     not abort the scan)

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Append `line` followed by a newline to `path`. Creates parent dirs (if any),
/// opens with `O_CREATE | O_APPEND`, enforces `0o600` permissions on Unix.
pub fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // On Unix, tighten loose permissions BEFORE opening for append so
    // concurrent writers never race against an in-flight `set_permissions`.
    // `OpenOptions::mode(0o600)` covers fresh creates; this branch handles
    // legacy files inherited with looser bits (audit.rs paranoia).
    #[cfg(unix)]
    if path.exists() {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let current = meta.permissions().mode() & 0o777;
            if current != 0o600 {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = fs::set_permissions(path, perms);
            }
        }
    }

    let mut opts = OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;

    writeln!(file, "{line}")?;
    Ok(())
}

/// Read every line from `path` and feed it through `parser`. Lines where the
/// parser returns `None` (decode failure, unknown schema, etc.) are skipped
/// silently. A missing file returns an empty `Vec` rather than an error so
/// readers can treat absence as "no events yet".
pub fn read_lines<T>(path: &Path, parser: impl Fn(&str) -> Option<T>) -> std::io::Result<Vec<T>> {
    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(value) = parser(line) {
            out.push(value);
        }
    }
    Ok(out)
}

/// Remove regular files under `dir` whose modification time is older than
/// `retain_days` days. Returns the number of files removed. A missing
/// directory or per-entry I/O failure is treated as a no-op (rotation must
/// never block writes); the caller can log if the count is surprising.
pub fn rotate_old(dir: &Path, retain_days: u64) -> std::io::Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retain_days.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let entries = match fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return Ok(0),
    };

    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(meta) = path.metadata() else {
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if mtime < cutoff && fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

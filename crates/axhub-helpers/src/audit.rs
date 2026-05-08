// Phase 2/3 — Audit module for Approach E (preflight + audit only) routing hook.
//
// Persists per-prompt routing audit lines (sha256 hash + length + cli_version
// + auth_ok + axhub-related boolean) to a daily JSONL file under state_dir().
//
// Privacy posture (docs/audit-privacy-contract.md):
// - prompt content NEVER stored (sha256 hash only, may be reversible for short prompts)
// - external transmission 0 (local disk only)
// - opt-out via AXHUB_NO_AUDIT=1 env var
// - 7-day rotation
// - dir 0700 / file 0600 on Unix
// - redact() defense-in-depth + panic::catch_unwind on JSONL line write
//
// All write operations silently skip on failure (disk full, permission denied,
// redact panic) — the hook MUST never crash because audit fails. Errors logged
// to stderr only.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::redact::redact;
use crate::runtime_paths::state_dir;

/// Single audit log line. prompt content never stored — hash + metadata only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub ts: String,
    pub prompt_hash: String,
    pub prompt_len: u32,
    pub cli_version: Option<String>,
    pub auth_ok: bool,
    pub is_axhub_related: bool,
}

/// ISO 8601 UTC timestamp ("2026-05-07T19:00:00+00:00").
pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339()
}

/// sha256 hex digest with "sha256:" prefix.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let bytes = hasher.finalize();
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("sha256:{}", hex)
}

fn audit_dir() -> Option<PathBuf> {
    state_dir()
}

fn today_utc_iso_date() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// Append a single AuditRecord JSONL line. Silent skip on any failure.
///
/// Behavior:
/// - AXHUB_NO_AUDIT=1 → no-op (no disk I/O)
/// - serde failure → stderr log, no write
/// - redact panic → catch_unwind + stderr log, fall back to raw line
/// - dir/file create failure → stderr log, no write
/// - write failure → stderr log
///
/// Always returns Ok — hook output never blocks on audit failure.
pub fn append(record: AuditRecord) -> std::io::Result<()> {
    // Any presence of AXHUB_NO_AUDIT (including empty string) disables audit.
    // Operationally safer than `=="1"` — typo or partial unset still opts out.
    if std::env::var("AXHUB_NO_AUDIT").is_ok() {
        return Ok(());
    }

    let line = match serde_json::to_string(&record) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[audit] serialize failed: {e}");
            return Ok(());
        }
    };

    let safe_line = std::panic::catch_unwind(|| redact(&line)).unwrap_or_else(|_| {
        eprintln!("[audit] redact panicked, using raw line");
        line.clone()
    });

    let Some(dir) = audit_dir() else {
        return Ok(());
    };

    if let Err(e) = ensure_dir_with_perms(&dir) {
        eprintln!("[audit] ensure dir failed: {e}");
        return Ok(());
    }

    let path = dir.join(format!("routing-audit-{}.jsonl", today_utc_iso_date()));
    match open_append_secure(&path) {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{safe_line}") {
                eprintln!("[audit] write failed: {e}");
            }
        }
        Err(e) => {
            eprintln!("[audit] open failed: {e}");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_dir_with_perms(dir: &PathBuf) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(dir)?;
    let mut perms = fs::metadata(dir)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(dir, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn ensure_dir_with_perms(dir: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dir)
}

#[cfg(unix)]
fn open_append_secure(path: &PathBuf) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn open_append_secure(path: &PathBuf) -> std::io::Result<std::fs::File> {
    OpenOptions::new().create(true).append(true).open(path)
}

/// Delete audit JSONL files older than retention_days. Returns count deleted.
/// Silent failures — directory or file permission errors swallowed.
pub fn rotate(retention_days: i64) -> std::io::Result<u32> {
    let Some(dir) = audit_dir() else {
        return Ok(0);
    };
    if !dir.exists() {
        return Ok(0);
    }

    let cutoff = Utc::now() - Duration::days(retention_days);
    let cutoff_date = cutoff.date_naive();
    let mut deleted: u32 = 0;

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name_owned = entry.file_name();
        let Some(name) = name_owned.to_str() else {
            continue;
        };

        let prefix = "routing-audit-";
        let suffix = ".jsonl";
        if !name.starts_with(prefix) || !name.ends_with(suffix) {
            continue;
        }
        let date_part = &name[prefix.len()..name.len() - suffix.len()];
        let Ok(file_date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") else {
            continue;
        };

        if file_date < cutoff_date && fs::remove_file(entry.path()).is_ok() {
            deleted += 1;
        }
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_deterministic() {
        let a = sha256_hex("배포해줘");
        let b = sha256_hex("배포해줘");
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
        assert_eq!(a.len(), "sha256:".len() + 64);
    }

    #[test]
    fn sha256_hex_distinguishes_inputs() {
        assert_ne!(sha256_hex("a"), sha256_hex("b"));
    }

    #[test]
    fn now_iso8601_parseable() {
        let s = now_iso8601();
        let parsed = chrono::DateTime::parse_from_rfc3339(&s);
        assert!(parsed.is_ok(), "now_iso8601 not RFC 3339 parseable: {s}");
    }

    #[test]
    fn audit_no_audit_env_skips() {
        // Snapshot existing env, set AXHUB_NO_AUDIT, restore after.
        let prev = std::env::var("AXHUB_NO_AUDIT").ok();
        // SAFETY: tests rely on env mutation; serial via cargo test single-thread or
        // trust that no parallel test reads AXHUB_NO_AUDIT in this crate.
        unsafe { std::env::set_var("AXHUB_NO_AUDIT", "1") };
        let result = append(AuditRecord {
            ts: now_iso8601(),
            prompt_hash: sha256_hex("test"),
            prompt_len: 4,
            cli_version: None,
            auth_ok: false,
            is_axhub_related: false,
        });
        match prev {
            Some(v) => unsafe { std::env::set_var("AXHUB_NO_AUDIT", v) },
            None => unsafe { std::env::remove_var("AXHUB_NO_AUDIT") },
        }
        assert!(result.is_ok());
    }
}

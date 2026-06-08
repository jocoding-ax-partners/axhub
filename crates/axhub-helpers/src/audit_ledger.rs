//! JSONL append-only audit ledger for the auto-diagnose system.
//!
//! Plan v6 §13.B (Phase 0b) — Probe manifest, fix attempts, postmortem entries
//! all go here. Distinct from `audit.rs` (routing-audit) and `telemetry.rs`
//! (phase markers). Uses `crate::atomic_jsonl` for atomic O_APPEND writes and
//! adds an fslock fence for operations that need cross-process serialization
//! (e.g. concurrent diagnose sessions across worktrees).
//!
//! Layout:
//!   <state_root>/audit-ledger/ledger.jsonl
//!   <state_root>/audit-ledger/.lock

use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::atomic_jsonl::{append_line, read_lines};
use crate::runtime_paths::state_root;

const LEDGER_DIR: &str = "audit-ledger";
const LEDGER_FILE: &str = "ledger.jsonl";
const LOCK_FILE: &str = ".lock";

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

/// A single audit ledger entry. `evidence` is a free-form JSON object that
/// each ledger kind documents in its own contract (e.g. probe manifest holds
/// `path`, `start_line`, `end_line`, `original_content_sha256`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerEntry {
    /// RFC3339 timestamp at entry creation.
    pub ts: String,
    /// Entry kind: `probe.apply`, `probe.revert`, `fix.apply`, `loop_verify`,
    /// `postmortem.cleanup`, `recurrence.threshold_hit`, etc.
    pub kind: String,
    /// Owning loop ID, if applicable.
    pub loop_id: Option<String>,
    /// Action being attempted (free-form, useful for traces).
    pub action: Option<String>,
    /// Hash of the cwd at entry creation (privacy-preserving correlation).
    pub cwd_hash: Option<String>,
    /// Kind-specific structured evidence.
    pub evidence: serde_json::Value,
}

impl LedgerEntry {
    /// Builder for a new entry with current UTC timestamp.
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            ts: Utc::now().to_rfc3339(),
            kind: kind.into(),
            loop_id: None,
            action: None,
            cwd_hash: None,
            evidence: serde_json::json!({}),
        }
    }

    pub fn with_loop_id(mut self, id: impl Into<String>) -> Self {
        self.loop_id = Some(id.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_cwd_hash(mut self, hash: impl Into<String>) -> Self {
        self.cwd_hash = Some(hash.into());
        self
    }

    pub fn with_evidence(mut self, evidence: serde_json::Value) -> Self {
        self.evidence = evidence;
        self
    }
}

fn ledger_dir() -> PathBuf {
    state_root().join(LEDGER_DIR)
}

/// Default ledger path. Tests can override via [`append_entry_to`].
pub fn ledger_path() -> PathBuf {
    ledger_dir().join(LEDGER_FILE)
}

fn lock_path() -> PathBuf {
    ledger_dir().join(LOCK_FILE)
}

/// Append `entry` to the default ledger. flock-fenced + O_APPEND atomic.
pub fn append_entry(entry: &LedgerEntry) -> Result<(), LedgerError> {
    append_entry_to(&ledger_path(), &lock_path(), entry)
}

/// Maximum time we wait to acquire the cross-process ledger lock before
/// giving up. Diagnose loops are user-facing (a frozen lock acquire would
/// make the CLI appear hung), so we surface a TimedOut error instead of
/// blocking forever. Override via `AXHUB_AUDIT_LOCK_TIMEOUT_MS` for tests.
pub const LEDGER_LOCK_TIMEOUT_MS: u64 = 5_000;
pub const LEDGER_LOCK_TIMEOUT_ENV: &str = "AXHUB_AUDIT_LOCK_TIMEOUT_MS";

fn effective_lock_timeout_ms() -> u64 {
    std::env::var(LEDGER_LOCK_TIMEOUT_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(LEDGER_LOCK_TIMEOUT_MS)
}

/// Acquire the fslock with a bounded poll. `fslock::LockFile::try_lock` is
/// non-blocking, so we sleep+retry until either the lock is held or the
/// timeout window expires. Returns Err(TimedOut) on expiry.
fn lock_with_timeout(lock_handle: &mut fslock::LockFile) -> Result<(), LedgerError> {
    let timeout = std::time::Duration::from_millis(effective_lock_timeout_ms());
    let started = std::time::Instant::now();
    loop {
        if lock_handle.try_lock()? {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(LedgerError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "audit ledger lock not acquired within {timeout:?}; another process \
                     may be holding the .lock file"
                ),
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// Append `entry` to an explicit ledger path. Used by tests + multi-tenant
/// scenarios that want their own ledger location.
pub fn append_entry_to(
    ledger: &std::path::Path,
    lock: &std::path::Path,
    entry: &LedgerEntry,
) -> Result<(), LedgerError> {
    let line = serde_json::to_string(entry)?;
    if let Some(parent) = lock.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut lock_handle = fslock::LockFile::open(lock)?;
    lock_with_timeout(&mut lock_handle)?;
    let res = append_line(ledger, &line);
    // Drop the lock explicitly. On all targets fslock also releases on Drop,
    // but an explicit drop documents intent and surfaces any unlock error
    // (which we still discard because the write already succeeded or failed).
    drop(lock_handle);
    res?;
    Ok(())
}

/// Read all entries from the default ledger. Corrupt lines silently skipped.
pub fn read_all() -> Result<Vec<LedgerEntry>, LedgerError> {
    read_all_from(&ledger_path())
}

/// Read all entries from an explicit ledger path.
pub fn read_all_from(path: &std::path::Path) -> Result<Vec<LedgerEntry>, LedgerError> {
    let lines = read_lines(path, |line| serde_json::from_str::<LedgerEntry>(line).ok())?;
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn paths(dir: &TempDir) -> (PathBuf, PathBuf) {
        (dir.path().join("ledger.jsonl"), dir.path().join(".lock"))
    }

    #[test]
    fn append_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let (ledger, lock) = paths(&dir);
        let entry = LedgerEntry::new("test.kind")
            .with_loop_id("loop-001")
            .with_action("apply")
            .with_cwd_hash("sha256:deadbeef")
            .with_evidence(serde_json::json!({"foo": "bar"}));
        append_entry_to(&ledger, &lock, &entry).unwrap();
        let entries = read_all_from(&ledger).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, "test.kind");
        assert_eq!(entries[0].loop_id.as_deref(), Some("loop-001"));
        assert_eq!(entries[0].evidence["foo"], "bar");
    }

    #[test]
    fn append_is_append_only() {
        let dir = TempDir::new().unwrap();
        let (ledger, lock) = paths(&dir);
        for i in 0..5 {
            let entry = LedgerEntry::new("test.kind").with_action(format!("act-{i}"));
            append_entry_to(&ledger, &lock, &entry).unwrap();
        }
        let entries = read_all_from(&ledger).unwrap();
        assert_eq!(entries.len(), 5);
        for (i, e) in entries.iter().enumerate() {
            assert_eq!(e.action.as_deref(), Some(format!("act-{i}").as_str()));
        }
    }

    #[test]
    fn corrupt_lines_skipped() {
        let dir = TempDir::new().unwrap();
        let (ledger, lock) = paths(&dir);
        let good = LedgerEntry::new("good.kind");
        append_entry_to(&ledger, &lock, &good).unwrap();
        // Pollute with malformed JSON
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&ledger)
            .unwrap();
        writeln!(f, "{{not valid json").unwrap();
        let good2 = LedgerEntry::new("good.kind2");
        append_entry_to(&ledger, &lock, &good2).unwrap();
        let entries = read_all_from(&ledger).unwrap();
        assert_eq!(entries.len(), 2, "corrupt line must be skipped");
        assert_eq!(entries[0].kind, "good.kind");
        assert_eq!(entries[1].kind, "good.kind2");
    }

    #[test]
    fn multi_thread_concurrent_appends() {
        use std::sync::Arc;
        use std::thread;
        let dir = Arc::new(TempDir::new().unwrap());
        let ledger = Arc::new(dir.path().join("ledger.jsonl"));
        let lock = Arc::new(dir.path().join(".lock"));
        let mut handles = vec![];
        let writer_count = 8;
        let per_writer = 5;
        for tid in 0..writer_count {
            let l = ledger.clone();
            let lk = lock.clone();
            handles.push(thread::spawn(move || {
                for i in 0..per_writer {
                    let entry = LedgerEntry::new("thread.kind").with_action(format!("t{tid}-{i}"));
                    append_entry_to(&l, &lk, &entry).unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let entries = read_all_from(&ledger).unwrap();
        assert_eq!(
            entries.len(),
            writer_count * per_writer,
            "all {} entries from {} threads must be present",
            writer_count * per_writer,
            writer_count,
        );
    }
}

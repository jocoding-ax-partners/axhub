// Phase 2 — file-system level integration tests for audit module.
//
// All tests scope IO into a tempdir via XDG_STATE_HOME so audit::audit_dir()
// resolves to ${tempdir}/axhub-plugin instead of the real ~/.local/state path.
// Tests run serially via the env mutation guard (single std::sync::Mutex).

use std::sync::Mutex;

use axhub_helpers::audit::{self, AuditRecord};
use chrono::{Duration, Utc};
use tempfile::TempDir;

// Env mutation must be single-threaded since XDG_STATE_HOME is process-global.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    prev_state: Option<String>,
    prev_no_audit: Option<String>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn new(temp: &TempDir) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_state = std::env::var("XDG_STATE_HOME").ok();
        let prev_no_audit = std::env::var("AXHUB_NO_AUDIT").ok();
        // SAFETY: tests serialize on ENV_LOCK; no concurrent reader within this crate.
        unsafe {
            std::env::set_var("XDG_STATE_HOME", temp.path());
            std::env::remove_var("AXHUB_NO_AUDIT");
        }
        EnvGuard {
            prev_state,
            prev_no_audit,
            _lock: lock,
        }
    }

    fn set_no_audit(&self) {
        unsafe { std::env::set_var("AXHUB_NO_AUDIT", "1") };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prev_state {
                Some(v) => std::env::set_var("XDG_STATE_HOME", v),
                None => std::env::remove_var("XDG_STATE_HOME"),
            }
            match &self.prev_no_audit {
                Some(v) => std::env::set_var("AXHUB_NO_AUDIT", v),
                None => std::env::remove_var("AXHUB_NO_AUDIT"),
            }
        }
    }
}

fn sample_record(label: &str) -> AuditRecord {
    AuditRecord {
        ts: audit::now_iso8601(),
        prompt_hash: audit::sha256_hex(label),
        prompt_len: label.len() as u32,
        cli_version: Some("0.1.0".into()),
        auth_ok: true,
        is_axhub_related: true,
    }
}

#[cfg(unix)]
#[test]
fn file_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    audit::append(sample_record("perm-test")).unwrap();

    let dir = temp.path().join("axhub-plugin");
    let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700, "audit dir mode {:o}", dir_mode);

    let mut entries = std::fs::read_dir(&dir).unwrap();
    let entry = entries
        .next()
        .expect("expected at least one audit file")
        .unwrap();
    let file_mode = std::fs::metadata(entry.path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(file_mode, 0o600, "audit file mode {:o}", file_mode);
}

#[test]
fn rotation_handles_today_yesterday_correctly() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();

    let stale_date = (Utc::now() - Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let yesterday = (Utc::now() - Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let today = Utc::now().format("%Y-%m-%d").to_string();

    let stale = dir.join(format!("routing-audit-{stale_date}.jsonl"));
    let yest = dir.join(format!("routing-audit-{yesterday}.jsonl"));
    let now = dir.join(format!("routing-audit-{today}.jsonl"));
    std::fs::write(&stale, "{}\n").unwrap();
    std::fs::write(&yest, "{}\n").unwrap();
    std::fs::write(&now, "{}\n").unwrap();

    let deleted = audit::rotate(7).unwrap();
    assert_eq!(deleted, 1, "expected only stale (8 days) deleted");
    assert!(!stale.exists());
    assert!(yest.exists());
    assert!(now.exists());
}

#[test]
fn audit_disabled_no_io() {
    let temp = tempfile::tempdir().unwrap();
    let g = EnvGuard::new(&temp);
    g.set_no_audit();

    audit::append(sample_record("no-io")).unwrap();

    let dir = temp.path().join("axhub-plugin");
    assert!(
        !dir.exists(),
        "audit dir must not be created when AXHUB_NO_AUDIT is set"
    );
}

#[test]
fn rotate_ignores_non_audit_and_malformed_audit_filenames() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let malformed = dir.join("routing-audit-not-a-date.jsonl");
    let unrelated = dir.join("notes.txt");
    std::fs::write(&malformed, "{}\n").unwrap();
    std::fs::write(&unrelated, "{}\n").unwrap();

    let deleted = audit::rotate(7).unwrap();

    assert_eq!(deleted, 0);
    assert!(malformed.exists());
    assert!(unrelated.exists());
}

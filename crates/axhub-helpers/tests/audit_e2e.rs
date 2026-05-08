// Phase 6 — audit_e2e.rs: file-system level integration tests for audit module.
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
        clarify_invoked: false,
        chosen_skill: None,
    }
}

#[test]
fn clarify_invoked_field_persists() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let mut record = sample_record("clarify-feedback");
    record.clarify_invoked = true;
    record.chosen_skill = Some("status".into());
    let expected_hash = record.prompt_hash.clone();

    audit::append(record).unwrap();

    let records = audit::read_since(Duration::days(1)).unwrap();
    let found = records
        .iter()
        .find(|r| r.prompt_hash == expected_hash)
        .expect("clarify feedback record");
    assert!(found.clarify_invoked);
    assert_eq!(found.chosen_skill.as_deref(), Some("status"));
}

#[test]
fn clarify_invoked_default_false_backward_compat() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let path = dir.join(format!("routing-audit-{today}.jsonl"));
    let legacy = serde_json::json!({
        "ts": audit::now_iso8601(),
        "prompt_hash": "sha256:legacy",
        "prompt_len": 12,
        "cli_version": "0.1.0",
        "auth_ok": true,
        "is_axhub_related": true
    });
    std::fs::write(&path, format!("{legacy}\n")).unwrap();

    let records = audit::read_since(Duration::days(1)).unwrap();
    let found = records
        .iter()
        .find(|r| r.prompt_hash == "sha256:legacy")
        .expect("legacy audit record");
    assert!(!found.clarify_invoked);
    assert!(found.chosen_skill.is_none());
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

#[cfg(unix)]
#[test]
fn append_reasserts_existing_file_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let file = dir.join(format!("routing-audit-{today}.jsonl"));
    std::fs::write(&file, "{}\n").unwrap();
    let mut loose = std::fs::metadata(&file).unwrap().permissions();
    loose.set_mode(0o644);
    std::fs::set_permissions(&file, loose).unwrap();

    audit::append(sample_record("existing-perm")).unwrap();

    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "existing audit file mode {:o}", mode);
}

#[test]
fn append_rotates_stale_files_on_normal_write_path() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let stale_date = (Utc::now() - Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let stale = dir.join(format!("routing-audit-{stale_date}.jsonl"));
    std::fs::write(&stale, "{}\n").unwrap();

    audit::append(sample_record("rotate-on-append")).unwrap();

    assert!(
        !stale.exists(),
        "normal audit append should enforce rotation"
    );
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
fn read_since_handles_corrupted_lines() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let path = dir.join(format!("routing-audit-{today}.jsonl"));

    let valid = serde_json::to_string(&sample_record("valid-1")).unwrap();
    let valid2 = serde_json::to_string(&sample_record("valid-2")).unwrap();
    let mut content = String::new();
    content.push_str(&valid);
    content.push('\n');
    content.push_str("{ corrupted-not-json }\n");
    content.push_str(&valid2);
    content.push('\n');
    content.push_str("\n"); // empty line, not corrupted
    std::fs::write(&path, content).unwrap();

    let records = audit::read_since(Duration::days(7)).unwrap();
    assert_eq!(
        records.len(),
        2,
        "expected 2 valid records, skipping corrupted"
    );
}

#[test]
fn audit_disabled_no_io() {
    let temp = tempfile::tempdir().unwrap();
    let g = EnvGuard::new(&temp);
    g.set_no_audit();

    audit::append(sample_record("no-io")).unwrap();

    // AXHUB_NO_AUDIT 비활성 → audit dir 자체 미생성.
    let dir = temp.path().join("axhub-plugin");
    assert!(
        !dir.exists(),
        "audit dir must not be created when AXHUB_NO_AUDIT is set"
    );
}

#[test]
#[ignore = "concurrent multi-process append is flake-prone in CI; nightly-only"]
fn concurrent_multi_process_append() {
    // POSIX O_APPEND atomic for write < PIPE_BUF. 2 thread 동시 append → no interleave.
    // Spec test: kept as ignored placeholder until a stable harness exists.
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let handles: Vec<_> = (0..2)
        .map(|i| {
            std::thread::spawn(move || {
                for _ in 0..50 {
                    let _ = audit::append(sample_record(&format!("worker-{i}")));
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    let records = audit::read_since(Duration::days(1)).unwrap();
    assert_eq!(records.len(), 100);
}

#[test]
fn cleanup_all_ignores_non_audit_files_and_deletes_audit_files() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let audit_file = dir.join(format!("routing-audit-{today}.jsonl"));
    let other_file = dir.join("profile.json");
    std::fs::write(&audit_file, "{}\n").unwrap();
    std::fs::write(&other_file, "{}\n").unwrap();

    let deleted = audit::cleanup_all().unwrap();

    assert_eq!(deleted, 1);
    assert!(!audit_file.exists());
    assert!(other_file.exists());
}

#[test]
fn read_since_skips_wrong_filenames_and_old_records() {
    let temp = tempfile::tempdir().unwrap();
    let _g = EnvGuard::new(&temp);

    let dir = temp.path().join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let path = dir.join(format!("routing-audit-{today}.jsonl"));

    let fresh = sample_record("fresh");
    let mut old = sample_record("old");
    old.ts = (Utc::now() - Duration::days(30)).to_rfc3339();

    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&fresh).unwrap(),
            serde_json::to_string(&old).unwrap()
        ),
    )
    .unwrap();
    std::fs::write(dir.join("routing-audit-not-a-date.jsonl"), "{}\n").unwrap();
    std::fs::write(dir.join("unrelated.jsonl"), "{}\n").unwrap();

    let records = audit::read_since(Duration::days(7)).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].prompt_hash, fresh.prompt_hash);
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

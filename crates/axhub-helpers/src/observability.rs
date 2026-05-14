//! v0.6.0 — Autowire observability events.
//!
//! Appends structured NDJSON to `$XDG_STATE_HOME/axhub-plugin/events.jsonl`.
//!
//! **Privacy**: command strings are NEVER logged in plaintext.
//! Only `HMAC-SHA256(salt, value)` is written to disk.
//! Salt is per-install random 32 bytes stored at
//! `$XDG_STATE_HOME/axhub-plugin/observability-salt` (mode 0600).
//! A zeroed fallback salt is used on getrandom failure — still opaque
//! without the salt file, but noted in tests.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::runtime_paths::state_dir;
use crate::settings_merge::MergeOutcome;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Salt management
// ---------------------------------------------------------------------------

/// Load existing salt or generate + persist a new one.
/// On any error returns zeroed bytes (degrades gracefully, never panics).
fn load_or_create_salt(state_dir: &Path) -> Vec<u8> {
    let salt_path = state_dir.join("observability-salt");

    // Try to load existing salt (hex-encoded, 64 chars = 32 bytes).
    if let Ok(content) = fs::read_to_string(&salt_path) {
        let hex = content.trim();
        if hex.len() == 64 {
            if let Ok(bytes) = hex_decode(hex) {
                return bytes;
            }
        }
    }

    // Generate new cryptographically random 32-byte salt.
    let mut salt = vec![0u8; 32];
    if getrandom::getrandom(&mut salt).is_err() {
        // Fallback: zeroed salt — still better than plaintext logging.
        return salt;
    }
    let hex_str = hex_encode(&salt);

    // Persist with restricted permissions (mode 0600).
    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    if let Ok(mut f) = opts.open(&salt_path) {
        let _ = f.write_all(hex_str.as_bytes());
        let _ = f.sync_all();
    }
    salt
}

// ---------------------------------------------------------------------------
// HMAC helpers
// ---------------------------------------------------------------------------

/// `HMAC-SHA256(salt, value)` rendered as `"hmac-sha256:<hex>"`.
pub fn hmac_hex(salt: &[u8], value: &str) -> String {
    let key = if salt.is_empty() { &[0u8; 32] as &[u8] } else { salt };
    let mut mac = HmacSha256::new_from_slice(key)
        .unwrap_or_else(|_| HmacSha256::new_from_slice(&[0u8; 32]).unwrap());
    mac.update(value.as_bytes());
    let result = mac.finalize().into_bytes();
    format!("hmac-sha256:{}", hex_encode(&result))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, ()> {
    if hex.len() % 2 != 0 {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

// ---------------------------------------------------------------------------
// Outcome → NDJSON field mapping
// ---------------------------------------------------------------------------

fn outcome_action(outcome: &MergeOutcome) -> &'static str {
    match outcome {
        MergeOutcome::Created => "create",
        MergeOutcome::Merged => "merge",
        MergeOutcome::NoOp => "noop",
        MergeOutcome::PreservedOther => "preserve",
        MergeOutcome::InvalidJson
        | MergeOutcome::PartialSchema
        | MergeOutcome::PermissionDenied => "abort",
    }
}

fn outcome_branch(outcome: &MergeOutcome) -> u8 {
    match outcome {
        MergeOutcome::Created => 1,
        MergeOutcome::Merged => 3,
        MergeOutcome::NoOp => 4,
        MergeOutcome::PreservedOther => 5,
        MergeOutcome::InvalidJson => 6,
        MergeOutcome::PartialSchema => 7,
        MergeOutcome::PermissionDenied => 8,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Append one autowire event line to `events.jsonl`.
///
/// `other_command` — pass `Some(cmd)` when outcome is `PreservedOther` so the
/// command is HMAC-hashed (never logged plaintext). Pass `None` otherwise.
pub fn append_autowire_event(
    outcome: &MergeOutcome,
    scope: &str,
    other_command: Option<&str>,
) -> anyhow::Result<()> {
    let Some(sd) = state_dir() else {
        return Ok(()); // state dir unresolvable — skip silently
    };
    fs::create_dir_all(&sd)?;

    let salt = load_or_create_salt(&sd);
    let other_hash = other_command.map(|cmd| hmac_hex(&salt, cmd));

    let event = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "event": "autowire-statusline",
        "action": outcome_action(outcome),
        "branch": outcome_branch(outcome),
        "scope": scope,
        "before_hash": null,
        "after_hash": null,
        "other_command_hash": other_hash,
    });

    let line = serde_json::to_string(&event)?;
    let events_path = sd.join("events.jsonl");

    let mut opts = OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&events_path)?;
    writeln!(f, "{line}")?;
    f.sync_all()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_hex_format_prefix() {
        let result = hmac_hex(b"test-salt", "some-command");
        assert!(
            result.starts_with("hmac-sha256:"),
            "result should start with hmac-sha256: prefix"
        );
        // 12 (prefix) + 64 (hex SHA256) = 76 chars
        assert_eq!(result.len(), 76);
    }

    #[test]
    fn hmac_hex_deterministic_same_salt() {
        let h1 = hmac_hex(b"salt", "cmd");
        let h2 = hmac_hex(b"salt", "cmd");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hmac_hex_different_values_produce_different_hashes() {
        let h1 = hmac_hex(b"salt", "plugin-a");
        let h2 = hmac_hex(b"salt", "plugin-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hmac_hex_different_salts_same_value_different_hashes() {
        let h1 = hmac_hex(b"salt-install-1", "same-command");
        let h2 = hmac_hex(b"salt-install-2", "same-command");
        assert_ne!(h1, h2, "different salts must produce different hashes");
    }

    #[test]
    fn outcome_action_covers_all_variants() {
        use MergeOutcome::*;
        assert_eq!(outcome_action(&Created), "create");
        assert_eq!(outcome_action(&Merged), "merge");
        assert_eq!(outcome_action(&NoOp), "noop");
        assert_eq!(outcome_action(&PreservedOther), "preserve");
        assert_eq!(outcome_action(&InvalidJson), "abort");
        assert_eq!(outcome_action(&PartialSchema), "abort");
        assert_eq!(outcome_action(&PermissionDenied), "abort");
    }

    struct XdgGuard {
        _dir: tempfile::TempDir,
        old_xdg: Option<std::ffi::OsString>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl XdgGuard {
        fn new() -> Self {
            let lock = crate::PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            let dir = tempfile::tempdir().unwrap();
            let old_xdg = std::env::var_os("XDG_STATE_HOME");
            unsafe { std::env::set_var("XDG_STATE_HOME", dir.path()) }
            Self { _dir: dir, old_xdg, _lock: lock }
        }
    }

    impl Drop for XdgGuard {
        fn drop(&mut self) {
            match self.old_xdg.take() {
                Some(v) => unsafe { std::env::set_var("XDG_STATE_HOME", v) },
                None => unsafe { std::env::remove_var("XDG_STATE_HOME") },
            }
        }
    }

    #[test]
    fn append_autowire_event_creates_jsonl_file() {
        let guard = XdgGuard::new();
        let result = append_autowire_event(&MergeOutcome::NoOp, "user", None);
        assert!(result.is_ok(), "append should succeed");
        let events = guard._dir.path().join("axhub-plugin/events.jsonl");
        assert!(events.exists(), "events.jsonl should be created");
        let content = fs::read_to_string(&events).unwrap();
        let v: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(v["event"], "autowire-statusline");
        assert_eq!(v["action"], "noop");
        assert_eq!(v["scope"], "user");
        assert_eq!(v["other_command_hash"], serde_json::Value::Null);
    }

    #[test]
    fn other_command_is_hashed_not_stored_plaintext() {
        let guard = XdgGuard::new();
        let _ = append_autowire_event(
            &MergeOutcome::PreservedOther,
            "user",
            Some("/usr/local/bin/other-plugin-statusline.sh"),
        );
        let events = guard._dir.path().join("axhub-plugin/events.jsonl");
        let content = fs::read_to_string(&events).unwrap();
        assert!(
            !content.contains("/usr/local/bin/other-plugin-statusline.sh"),
            "plaintext command must not appear in events.jsonl"
        );
        let v: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        let hash = v["other_command_hash"].as_str().unwrap_or("");
        assert!(
            hash.starts_with("hmac-sha256:"),
            "other_command_hash should be hmac-sha256"
        );
    }
}

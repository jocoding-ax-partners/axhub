//! Phase 3 — `axhub-helpers config get/set` for user preferences.
//!
//! Spec: `.plan/deploy-time-reduction/phase-3-client-cascade-reduced.md` §4.2.
//!
//! Stores `~/.config/axhub-plugin/preferences.json` (JSON instead of TOML —
//! reuses serde_json from existing dependencies, no new crate). Atomic write
//! via tempfile + rename, unix mode 0600. Single supported key for now:
//! `ignore_too_new_until` (semver string suppressing the cli-too-new prompt).
//! `AXHUB_CLI_TOO_NEW_DISMISS=0` short-circuits `config_get` so the kill
//! switch always re-enables prompting.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const PREFERENCES_FILENAME: &str = "preferences.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CliPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_too_new_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Preferences {
    #[serde(default)]
    pub cli: CliPreferences,
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn preferences_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"));
    base.join("axhub-plugin").join(PREFERENCES_FILENAME)
}

fn read_preferences_at(path: &Path) -> Preferences {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Preferences>(&raw).ok())
        .unwrap_or_default()
}

fn write_preferences_at(path: &Path, prefs: &Preferences) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let tmp_path = path.with_extension("json.tmp");
    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp_path)?;
    let json = serde_json::to_string_pretty(prefs)?;
    f.write_all(json.as_bytes())?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    drop(f);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn dismiss_disabled() -> bool {
    std::env::var("AXHUB_CLI_TOO_NEW_DISMISS").as_deref() == Ok("0")
}

pub fn config_get(key: &str) -> Option<String> {
    config_get_at(key, &preferences_path())
}

pub fn config_get_at(key: &str, path: &Path) -> Option<String> {
    if dismiss_disabled() {
        return None;
    }
    let prefs = read_preferences_at(path);
    match key {
        "ignore_too_new_until" => prefs.cli.ignore_too_new_until,
        _ => None,
    }
}

pub fn config_set(key: &str, value: &str) -> anyhow::Result<()> {
    config_set_at(key, value, &preferences_path())
}

pub fn config_set_at(key: &str, value: &str, path: &Path) -> anyhow::Result<()> {
    let mut prefs = read_preferences_at(path);
    match key {
        "ignore_too_new_until" => {
            prefs.cli.ignore_too_new_until = Some(value.to_string());
        }
        other => {
            anyhow::bail!("unknown config key: {other}");
        }
    }
    write_preferences_at(path, &prefs)
}

/// Render a `config get` JSON envelope; preserves the same shape whether
/// the key is set, missing, or suppressed by the kill switch.
pub fn render_get_json(key: &str, value: Option<&str>) -> String {
    let mut obj = Map::new();
    obj.insert("key".into(), Value::String(key.to_string()));
    obj.insert(
        "value".into(),
        match value {
            Some(v) => Value::String(v.to_string()),
            None => Value::Null,
        },
    );
    Value::Object(obj).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_prefs_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "axhub-prefs-{}-{}.json",
            std::process::id(),
            uuid_like()
        ))
    }

    fn uuid_like() -> u64 {
        // cheap nonce — avoid pulling in `uuid` for a unit test
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        nanos.wrapping_mul(0x9e37_79b9_7f4a_7c15)
    }

    #[test]
    fn config_set_and_get_roundtrip() {
        let path = tmp_prefs_path();
        let _ = std::fs::remove_file(&path);
        config_set_at("ignore_too_new_until", "v0.12.5", &path).unwrap();
        assert_eq!(
            config_get_at("ignore_too_new_until", &path),
            Some("v0.12.5".to_string())
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn config_get_returns_none_when_file_missing() {
        let path = tmp_prefs_path();
        let _ = std::fs::remove_file(&path);
        assert_eq!(config_get_at("ignore_too_new_until", &path), None);
    }

    #[test]
    fn config_set_unknown_key_errors() {
        let path = tmp_prefs_path();
        let _ = std::fs::remove_file(&path);
        let err = config_set_at("not_a_real_key", "x", &path).unwrap_err();
        assert!(err.to_string().contains("unknown config key"));
    }

    #[test]
    fn render_get_json_emits_null_for_missing_value() {
        let json = render_get_json("ignore_too_new_until", None);
        assert!(json.contains("\"value\":null"));
        assert!(json.contains("\"key\":\"ignore_too_new_until\""));
    }
}

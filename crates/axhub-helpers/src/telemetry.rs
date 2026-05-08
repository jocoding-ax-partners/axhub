use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HELPER_VERSION: &str = env!("CARGO_PKG_VERSION");
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)").unwrap());
const CLI_VERSION_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct CachedCliVersion {
    version: String,
    resolved_at: Instant,
}

static CACHED_CLI_VERSION: Mutex<Option<CachedCliVersion>> = Mutex::new(None);

static PHASE_MARKERS: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

pub fn record_phase_marker(phase_name: &str) {
    if let Ok(mut markers) = PHASE_MARKERS.lock() {
        markers.push((phase_name.to_string(), Instant::now()));
    }
}

pub fn reset_phase_markers() {
    if let Ok(mut markers) = PHASE_MARKERS.lock() {
        markers.clear();
    }
}

fn compute_phase_durations(markers: &[(String, Instant)]) -> Map<String, Value> {
    let mut out = Map::new();
    if markers.len() < 2 {
        return out;
    }
    for window in markers.windows(2) {
        let (name_a, instant_a) = &window[0];
        let (name_b, instant_b) = &window[1];
        let duration_ms = instant_b.saturating_duration_since(*instant_a).as_millis() as u64;
        out.insert(
            format!("{}_to_{}", name_a, name_b),
            Value::Number(duration_ms.into()),
        );
    }
    out
}

fn drain_phase_durations_ms() -> Map<String, Value> {
    let Ok(mut markers) = PHASE_MARKERS.lock() else {
        return Map::new();
    };
    let durations = compute_phase_durations(&markers);
    markers.clear();
    durations
}

pub fn emit_deploy_complete(exit_code: i32, command_class: &str) -> anyhow::Result<()> {
    let mut fields = Map::new();
    fields.insert("event".into(), Value::String("deploy_complete".into()));
    fields.insert("exit_code".into(), Value::Number(exit_code.into()));
    fields.insert(
        "command_class".into(),
        Value::String(command_class.into()),
    );
    let durations = drain_phase_durations_ms();
    if !durations.is_empty() {
        fields.insert("phase_durations_ms".into(), Value::Object(durations));
    }
    emit_meta_envelope(fields)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve_cli_version() -> String {
    resolve_cli_version_with(Instant::now(), resolve_uncached_cli_version)
}

fn resolve_cli_version_with<F>(now: Instant, mut resolve_uncached: F) -> String
where
    F: FnMut() -> String,
{
    let mut cache = CACHED_CLI_VERSION.lock().expect("cli version cache lock");
    if let Some(cached) = cache.as_ref() {
        let fresh = now.saturating_duration_since(cached.resolved_at) < CLI_VERSION_CACHE_TTL;
        if fresh && cached.version == HELPER_VERSION {
            return cached.version.clone();
        }
    }
    let resolved = resolve_uncached();
    *cache = Some(CachedCliVersion {
        version: resolved.clone(),
        resolved_at: now,
    });
    resolved
}

fn read_cli_version_from(command: &str) -> Option<String> {
    Command::new(command)
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                VERSION_RE.captures(&text).map(|c| c[1].to_string())
            } else {
                None
            }
        })
}

fn resolve_uncached_cli_version() -> String {
    #[cfg(windows)]
    {
        for command in ["axhub", "axhub.exe", "axhub.cmd", "axhub.bat"] {
            if let Some(version) = read_cli_version_from(command) {
                return version;
            }
        }
        "unknown".into()
    }

    #[cfg(not(windows))]
    {
        read_cli_version_from("axhub").unwrap_or_else(|| "unknown".into())
    }
}

pub fn reset_cli_version_cache() {
    *CACHED_CLI_VERSION.lock().expect("cli version cache lock") = None;
}

pub fn is_enabled() -> bool {
    std::env::var("AXHUB_TELEMETRY").as_deref() == Ok("1")
}
pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/state"))
        .join("axhub-plugin")
}

pub fn emit_meta_envelope(fields: Map<String, Value>) -> anyhow::Result<()> {
    if !is_enabled() {
        return Ok(());
    }
    let result = (|| -> anyhow::Result<()> {
        let dir = state_dir();
        fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
        }
        let mut envelope = Map::new();
        envelope.insert(
            "ts".into(),
            Value::String(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
        );
        envelope.insert(
            "session_id".into(),
            Value::String(
                std::env::var("CLAUDE_SESSION_ID")
                    .or_else(|_| std::env::var("CLAUDECODE_SESSION_ID"))
                    .unwrap_or_else(|_| "unknown".into()),
            ),
        );
        envelope.insert(
            "plugin_version".into(),
            Value::String(PLUGIN_VERSION.into()),
        );
        envelope.insert("cli_version".into(), Value::String(resolve_cli_version()));
        envelope.insert(
            "helper_version".into(),
            Value::String(HELPER_VERSION.into()),
        );
        for (k, v) in fields {
            envelope.insert(k, v);
        }
        let file = dir.join("usage.jsonl");
        let mut opts = OpenOptions::new();
        opts.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&file)?;
        writeln!(f, "{}", Value::Object(envelope))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&file, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    })();
    // Telemetry must not block the hot path; match TS by swallowing failures.
    let _ = result;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::{Mutex, OnceLock};

    fn cli_version_cache_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn cli_version_cache_uses_thirty_second_ttl_for_matching_helper_version() {
        let _guard = cli_version_cache_test_lock().lock().unwrap();
        reset_cli_version_cache();
        let calls = Cell::new(0);
        let start = Instant::now();

        let first = resolve_cli_version_with(start, || {
            calls.set(calls.get() + 1);
            HELPER_VERSION.to_string()
        });
        assert_eq!(first, HELPER_VERSION);

        let second = resolve_cli_version_with(start + Duration::from_secs(29), || {
            calls.set(calls.get() + 1);
            "9.9.9".into()
        });
        assert_eq!(second, HELPER_VERSION);
        assert_eq!(calls.get(), 1);

        let third = resolve_cli_version_with(start + Duration::from_secs(30), || {
            calls.set(calls.get() + 1);
            "9.9.9".into()
        });
        assert_eq!(third, "9.9.9");
        assert_eq!(calls.get(), 2);
        reset_cli_version_cache();
    }

    #[test]
    fn compute_phase_durations_empty_when_zero_or_one_marker() {
        let zero: Vec<(String, Instant)> = vec![];
        assert!(compute_phase_durations(&zero).is_empty());
        let one = vec![("a".to_string(), Instant::now())];
        assert!(compute_phase_durations(&one).is_empty());
    }

    #[test]
    fn compute_phase_durations_records_sliding_window_pairs() {
        let t0 = Instant::now();
        let markers = vec![
            ("step_a".to_string(), t0),
            ("step_b".to_string(), t0 + Duration::from_millis(120)),
            ("step_c".to_string(), t0 + Duration::from_millis(420)),
        ];
        let out = compute_phase_durations(&markers);
        assert_eq!(out.len(), 2);
        assert_eq!(out.get("step_a_to_step_b").and_then(|v| v.as_u64()), Some(120));
        assert_eq!(out.get("step_b_to_step_c").and_then(|v| v.as_u64()), Some(300));
    }

    #[test]
    fn record_and_drain_phase_markers_roundtrip() {
        reset_phase_markers();
        record_phase_marker("alpha");
        std::thread::sleep(Duration::from_millis(2));
        record_phase_marker("beta");
        let drained = drain_phase_durations_ms();
        assert_eq!(drained.len(), 1);
        assert!(drained.contains_key("alpha_to_beta"));
        let again = drain_phase_durations_ms();
        assert!(again.is_empty());
    }

    #[test]
    fn cli_version_cache_invalidates_cached_helper_mismatch_immediately() {
        let _guard = cli_version_cache_test_lock().lock().unwrap();
        reset_cli_version_cache();
        let calls = Cell::new(0);
        let start = Instant::now();

        let first = resolve_cli_version_with(start, || {
            calls.set(calls.get() + 1);
            "0.0.1".into()
        });
        assert_eq!(first, "0.0.1");

        let second = resolve_cli_version_with(start + Duration::from_secs(1), || {
            calls.set(calls.get() + 1);
            HELPER_VERSION.to_string()
        });
        assert_eq!(second, HELPER_VERSION);
        assert_eq!(calls.get(), 2);
        reset_cli_version_cache();
    }
}

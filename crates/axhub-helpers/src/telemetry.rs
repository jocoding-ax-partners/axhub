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

/// Append a phase marker line to `path`.
///
/// Marker entries are wall-clock derived (`ns` is `SystemTime::now() - UNIX_EPOCH`
/// in nanoseconds) and serialized as one NDJSON line per call. POSIX guarantees
/// `O_APPEND` writes ≤PIPE_BUF (≥4 KiB) are atomic across concurrent writers.
///
/// Wall clock implies that NTP slew or `clock_settime` between two marks can
/// produce a non-monotonic pair. `drain_file_phase_durations` clamps negative
/// pairs to zero and reports the count via the `_backwards_skips` field.
pub fn append_phase_marker_to_file(path: &std::path::Path, phase_name: &str) -> anyhow::Result<()> {
    let mut entry = Map::new();
    entry.insert("name".into(), Value::String(phase_name.to_string()));
    entry.insert(
        "ns".into(),
        Value::Number(serde_json::Number::from(epoch_nanos())),
    );
    entry.insert(
        "ts".into(),
        Value::String(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
    );
    entry.insert("clock_source".into(), Value::String("wall".into()));
    let line = Value::Object(entry).to_string();
    // Phase 26 PR 26.1a — delegate to atomic_jsonl. Behavior-preserving:
    // same parent-dir creation, same O_CREATE | O_APPEND, same 0o600 perms.
    crate::atomic_jsonl::append_line(path, &line)?;
    Ok(())
}

fn epoch_nanos() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Drain the marker NDJSON at `path` into a `phase_durations_ms` map.
///
/// Single-writer assumption: callers must ensure no concurrent
/// `append_phase_marker_to_file` runs while drain executes. The harness
/// satisfies this by calling `emit-deploy-complete` only after every
/// `mark` invocation has returned.
///
/// Pairs whose end timestamp precedes their start timestamp (NTP backwards
/// shift, system clock change) are clamped to `0` and counted in the
/// `_backwards_skips` field so consumers can detect noise rather than
/// silently treating negative durations as zero.
pub fn drain_file_phase_durations(path: &std::path::Path) -> Map<String, Value> {
    let mut out = Map::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return out;
    };
    let mut entries: Vec<(String, u64)> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(name) = parsed.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(ns) = parsed.get("ns").and_then(|v| v.as_u64()) else {
            continue;
        };
        entries.push((name.to_string(), ns));
    }
    let mut backwards_skips: u64 = 0;
    if entries.len() >= 2 {
        for window in entries.windows(2) {
            let (name_a, ns_a) = &window[0];
            let (name_b, ns_b) = &window[1];
            let duration_ms = if ns_b >= ns_a {
                (ns_b - ns_a) / 1_000_000
            } else {
                backwards_skips += 1;
                0
            };
            out.insert(
                format!("{}_to_{}", name_a, name_b),
                Value::Number(duration_ms.into()),
            );
        }
    }
    if backwards_skips > 0 {
        out.insert(
            "_backwards_skips".into(),
            Value::Number(backwards_skips.into()),
        );
    }
    let _ = fs::remove_file(path);
    out
}

pub fn emit_deploy_complete(exit_code: i32, command_class: &str) -> anyhow::Result<()> {
    let mut fields = Map::new();
    fields.insert("event".into(), Value::String("deploy_complete".into()));
    fields.insert("exit_code".into(), Value::Number(exit_code.into()));
    fields.insert("command_class".into(), Value::String(command_class.into()));
    let durations = match std::env::var("AXHUB_PHASE_MARKER_FILE") {
        Ok(p) if !p.is_empty() => {
            let file_durations = drain_file_phase_durations(std::path::Path::new(&p));
            // Always drain in-memory too so subsequent calls don't see stale markers,
            // but file-backed wins when both present.
            let _ = drain_phase_durations_ms();
            file_durations
        }
        _ => drain_phase_durations_ms(),
    };
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
        assert_eq!(
            out.get("step_a_to_step_b").and_then(|v| v.as_u64()),
            Some(120)
        );
        assert_eq!(
            out.get("step_b_to_step_c").and_then(|v| v.as_u64()),
            Some(300)
        );
    }

    #[test]
    fn append_and_drain_file_phase_markers_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("axhub-marker-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        append_phase_marker_to_file(&tmp, "alpha").unwrap();
        std::thread::sleep(Duration::from_millis(2));
        append_phase_marker_to_file(&tmp, "beta").unwrap();
        let raw = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(raw.trim().lines().count(), 2);
        assert!(raw.contains("\"name\":\"alpha\""));
        assert!(raw.contains("\"name\":\"beta\""));

        let drained = drain_file_phase_durations(&tmp);
        assert_eq!(drained.len(), 1);
        assert!(drained.contains_key("alpha_to_beta"));
        // file removed on drain
        assert!(!tmp.exists());

        // second drain on missing path → empty
        let again = drain_file_phase_durations(&tmp);
        assert!(again.is_empty());
    }

    #[test]
    fn drain_file_phase_durations_clamps_backwards_pair_and_reports_skips() {
        let tmp =
            std::env::temp_dir().join(format!("axhub-marker-back-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        std::fs::write(
            &tmp,
            "{\"name\":\"a\",\"ns\":3000000}\n{\"name\":\"b\",\"ns\":1000000}\n",
        )
        .unwrap();
        let out = drain_file_phase_durations(&tmp);
        assert_eq!(out.get("a_to_b").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(
            out.get("_backwards_skips").and_then(|v| v.as_u64()),
            Some(1)
        );
    }

    #[test]
    fn emit_deploy_complete_writes_phase_durations_via_file_drain() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let prev_telemetry = std::env::var("AXHUB_TELEMETRY").ok();
        let prev_marker = std::env::var("AXHUB_PHASE_MARKER_FILE").ok();
        let prev_xdg = std::env::var("XDG_STATE_HOME").ok();

        let pid = std::process::id();
        let workdir = std::env::temp_dir().join(format!("axhub-emit-{pid}"));
        let _ = std::fs::remove_dir_all(&workdir);
        std::fs::create_dir_all(&workdir).unwrap();
        let marker_path = workdir.join("markers.jsonl");
        let state_root = workdir.join("state");

        std::env::set_var("AXHUB_TELEMETRY", "1");
        std::env::set_var("AXHUB_PHASE_MARKER_FILE", &marker_path);
        std::env::set_var("XDG_STATE_HOME", &state_root);

        append_phase_marker_to_file(&marker_path, "alpha").unwrap();
        std::thread::sleep(Duration::from_millis(2));
        append_phase_marker_to_file(&marker_path, "beta").unwrap();
        emit_deploy_complete(0, "test").unwrap();

        let usage = state_root.join("axhub-plugin").join("usage.jsonl");
        assert!(usage.exists(), "usage.jsonl missing at {:?}", usage);
        let raw = std::fs::read_to_string(&usage).unwrap();
        let last_line = raw.lines().last().expect("usage.jsonl had no lines");
        let parsed: serde_json::Value = serde_json::from_str(last_line).unwrap();
        assert_eq!(
            parsed.get("event").and_then(|v| v.as_str()),
            Some("deploy_complete")
        );
        assert!(parsed.get("phase_durations_ms").is_some());
        let durations = parsed.get("phase_durations_ms").unwrap();
        assert!(durations.get("alpha_to_beta").is_some());

        // marker file consumed
        assert!(!marker_path.exists());

        let _ = std::fs::remove_dir_all(&workdir);
        match prev_telemetry {
            Some(v) => std::env::set_var("AXHUB_TELEMETRY", v),
            None => std::env::remove_var("AXHUB_TELEMETRY"),
        }
        match prev_marker {
            Some(v) => std::env::set_var("AXHUB_PHASE_MARKER_FILE", v),
            None => std::env::remove_var("AXHUB_PHASE_MARKER_FILE"),
        }
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_STATE_HOME", v),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
    }

    #[test]
    fn drain_file_phase_durations_skips_corrupt_lines() {
        let tmp =
            std::env::temp_dir().join(format!("axhub-marker-corrupt-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        std::fs::write(
            &tmp,
            "{\"name\":\"a\",\"ns\":1000000}\nNOT_JSON\n{\"name\":\"b\",\"ns\":3000000}\n",
        )
        .unwrap();
        let out = drain_file_phase_durations(&tmp);
        assert_eq!(out.len(), 1);
        assert_eq!(out.get("a_to_b").and_then(|v| v.as_u64()), Some(2));
    }

    #[test]
    fn record_and_drain_phase_markers_roundtrip() {
        // Serialize against `emit_deploy_complete_writes_phase_durations_via_file_drain`
        // which also drains the in-memory PHASE_MARKERS as a side effect when the
        // file marker env is set.
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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

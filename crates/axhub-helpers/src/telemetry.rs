use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HELPER_VERSION: &str = env!("CARGO_PKG_VERSION");
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)").unwrap());
static CACHED_CLI_VERSION: Mutex<Option<String>> = Mutex::new(None);

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve_cli_version() -> String {
    let mut cache = CACHED_CLI_VERSION.lock().expect("cli version cache lock");
    if let Some(v) = cache.as_ref() {
        return v.clone();
    }
    let resolved = resolve_uncached_cli_version();
    *cache = Some(resolved.clone());
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

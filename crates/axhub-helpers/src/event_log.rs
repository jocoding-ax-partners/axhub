// Phase 26 PR 26.1b — append-only deploy audit trail.
//
// Each `axhub deploy create` invocation generates a fresh `deploy_id`. As
// the lifecycle progresses (preflight → resolve → bootstrap → push → verify
// → completed/failed) we append one NDJSON line per phase transition to
// `$XDG_STATE_HOME/axhub-plugin/deploy-events/{deploy_id}.jsonl`.
//
// Plan reference:
// - .plan/matrix-absorption/phases/phase-26-tier-s-quick-wins.md PR 26.1b
// - schema_version pinned to "deploy-event/v1" so PR 26.2 (phase machine)
//   and PR 25.1 (recovery scan) can guard against forward-incompatible
//   versions without re-deriving the contract.
//
// Privacy + safety contract:
// - `AXHUB_DISABLE_DEPLOY_EVENTS=1` → write is a silent no-op (matches the
//   env taxonomy ADR §10.6: destructive opt-out via `AXHUB_DISABLE_*=1`).
// - File perms 0o600 / dir created with parent stack via atomic_jsonl.
// - Reads are fail-soft: corrupt or unknown-schema lines are skipped, never
//   propagated as errors.
// - List operations short-circuit on a missing directory (`Ok(vec![])`).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::atomic_jsonl;
use crate::runtime_paths::deploy_events_dir;

/// Pinned NDJSON envelope version. Bump only with a coordinated migration —
/// readers MUST skip unknown values rather than panic so an older binary
/// staying compatible with a newer log is the default.
pub const EVENT_LOG_SCHEMA_VERSION: &str = "deploy-event/v1";

/// One deploy phase transition. The struct intentionally mirrors the plan
/// spec column-for-column so PR 25.1 recovery scan + PR 25.4 trace skill can
/// project this verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployEvent {
    pub schema_version: String,
    pub deploy_id: String,
    /// ISO 8601 with millisecond precision, UTC ("Z" suffix).
    pub ts: String,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
}

impl DeployEvent {
    /// Convenience constructor that stamps `schema_version` + a fresh ISO
    /// timestamp so callers don't have to remember.
    pub fn new(deploy_id: impl Into<String>, phase: impl Into<String>) -> Self {
        Self {
            schema_version: EVENT_LOG_SCHEMA_VERSION.to_string(),
            deploy_id: deploy_id.into(),
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            phase: phase.into(),
            exit_code: None,
            duration_ms: None,
            reason: None,
            evidence: None,
        }
    }
}

/// Error surface kept small so callers can pattern-match without pulling in
/// `anyhow` everywhere. `PathResolution` covers the rare environments where
/// `XDG_STATE_HOME` + `HOME` are both unset.
#[derive(Debug, Error)]
pub enum EventLogError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("path resolution failed (XDG_STATE_HOME + HOME both unset?)")]
    PathResolution,
    #[error("invalid deploy_id (must be a single filename segment): {0}")]
    InvalidDeployId(String),
}

/// Append a single event to the per-deploy NDJSON log. Returns `Ok(())` when
/// the opt-out env is set so callers can treat audit as best-effort.
pub fn append_event(deploy_id: &str, event: &DeployEvent) -> Result<(), EventLogError> {
    if disabled_via_env() {
        return Ok(());
    }
    let path = log_path(deploy_id)?;
    let line = serde_json::to_string(event)?;
    atomic_jsonl::append_line(&path, &line)?;
    Ok(())
}

/// Read every event for a given deploy_id. Corrupt lines and unknown
/// `schema_version` rows are skipped silently — readers MUST keep working
/// when a newer writer ships incompatible data.
pub fn read_events(deploy_id: &str) -> Result<Vec<DeployEvent>, EventLogError> {
    let path = log_path(deploy_id)?;
    let events = atomic_jsonl::read_lines(&path, |line| {
        let parsed: DeployEvent = serde_json::from_str(line).ok()?;
        if parsed.schema_version == EVENT_LOG_SCHEMA_VERSION {
            Some(parsed)
        } else {
            None
        }
    })?;
    Ok(events)
}

/// Return `deploy_id`s whose log file was modified within the last
/// `within_secs` seconds. Used by PR 25.1 recovery scan to find "still
/// in-flight" deploys after a crash.
pub fn list_recent_deploys(within_secs: u64) -> Result<Vec<String>, EventLogError> {
    let Some(dir) = deploy_events_dir() else {
        return Err(EventLogError::PathResolution);
    };
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(within_secs))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut deploys: Vec<(String, SystemTime)> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(deploy_id) = name.strip_suffix(".jsonl") else {
            continue;
        };
        // Tolerate per-entry stat failures — they fall out of the window.
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if mtime >= cutoff {
            deploys.push((deploy_id.to_string(), mtime));
        }
    }

    // Newest first — recovery scan picks the most recent incomplete deploy.
    deploys.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(deploys.into_iter().map(|(id, _)| id).collect())
}

/// Derived-view helper: most recent phase name for the deploy, or `None` if
/// the log is empty or missing. Used by PR 26.2 spike option (b) — the
/// "event-sourced FSM" alternative needs zero new state to compute current
/// phase.
pub fn current_phase(deploy_id: &str) -> Option<String> {
    read_events(deploy_id)
        .ok()
        .and_then(|events| events.last().map(|e| e.phase.clone()))
}

fn log_path(deploy_id: &str) -> Result<PathBuf, EventLogError> {
    validate_deploy_id(deploy_id)?;
    let dir = deploy_events_dir().ok_or(EventLogError::PathResolution)?;
    Ok(dir.join(format!("{deploy_id}.jsonl")))
}

fn validate_deploy_id(deploy_id: &str) -> Result<(), EventLogError> {
    if deploy_id.is_empty()
        || deploy_id == "."
        || deploy_id == ".."
        || !deploy_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err(EventLogError::InvalidDeployId(deploy_id.to_string()));
    }
    Ok(())
}

fn disabled_via_env() -> bool {
    matches!(
        std::env::var("AXHUB_DISABLE_DEPLOY_EVENTS").as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

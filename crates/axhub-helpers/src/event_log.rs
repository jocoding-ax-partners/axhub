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

// =============================================================================
// Phase 26 PR 26.2 — phase logic (event-sourcing derived view).
//
// The plan ran a 1-week spike (overview §10.2 #10) comparing two abstractions
// for the deploy lifecycle:
//   option (a) `phase_machine.rs` — first-class FSM, ~120 LOC + ~250 LOC test,
//               stateful Failed→Idle reset requiring an explicit `axhub reset`
//               subcommand (Tier B #16 prerequisite), stateless principle
//               amend mandatory.
//   option (b) event-sourcing derived view — DeployPhase enum + transition
//               table on top of `current_phase()` (~80 LOC including tests),
//               single source of truth is the NDJSON log, Failed→Idle is
//               implicit (the next user-triggered deploy_id starts a fresh
//               event chain — "last event wins"), no stateless amend.
//
// Outcome: option (b) chosen. Drivers per plan §PR 26.2 weak preference:
//   - LOC ↓ (~80 vs ~370)
//   - stateless ↑ (no cross-process Failed state to keep coherent)
//   - natural integration with the existing `statusline::is_terminal_phase`
//     terminal-string set
// Tier B #16 (`axhub reset` subcommand) stays P3 — recover skill already
// covers the "user explicitly wants to clear failed state" path because
// option (b) treats a fresh Preflight event as the implicit reset.
//
// Stateless §10.7 reconciliation: derived view is fully stateless — every
// "current phase" answer is computed from the append-only event log. The
// overview §3 non-goal needs NO amend.

/// Coarse lifecycle of an `axhub deploy create` invocation. Mirrors plan
/// §PR 26.2 transition table; the enum is purely a derived view computed by
/// reading the event log, never stored anywhere as a separate piece of state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployPhase {
    Idle,
    Preflight,
    Resolve,
    Bootstrap,
    Push,
    Verify,
    Completed,
    Failed,
    Aborted,
}

impl DeployPhase {
    /// Canonical lowercase snake_case name used in the NDJSON `phase` field.
    pub fn as_str(&self) -> &'static str {
        match self {
            DeployPhase::Idle => "idle",
            DeployPhase::Preflight => "preflight",
            DeployPhase::Resolve => "resolve",
            DeployPhase::Bootstrap => "bootstrap",
            DeployPhase::Push => "push",
            DeployPhase::Verify => "verify",
            DeployPhase::Completed => "completed",
            DeployPhase::Failed => "failed",
            DeployPhase::Aborted => "aborted",
        }
    }

    /// Parse a phase string emitted by an `axhub` CLI or our own event log.
    /// Returns `None` for unrecognized values so callers can fail soft.
    pub fn parse(s: &str) -> Option<DeployPhase> {
        match s {
            "idle" => Some(DeployPhase::Idle),
            "preflight" => Some(DeployPhase::Preflight),
            "resolve" => Some(DeployPhase::Resolve),
            "bootstrap" => Some(DeployPhase::Bootstrap),
            "push" => Some(DeployPhase::Push),
            "verify" => Some(DeployPhase::Verify),
            "completed" | "complete" | "succeeded" | "success" => Some(DeployPhase::Completed),
            "failed" | "errored" => Some(DeployPhase::Failed),
            "aborted" | "cancelled" | "canceled" => Some(DeployPhase::Aborted),
            _ => None,
        }
    }

    /// Terminal phases never auto-progress; a fresh `Preflight` event
    /// implicitly starts the next attempt (last-event-wins).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DeployPhase::Completed | DeployPhase::Failed | DeployPhase::Aborted
        )
    }

    /// Plan §PR 26.2 transition table. `Failed` / `Aborted` → `Preflight`
    /// reset is allowed because, under last-event-wins semantics, the user
    /// kicking off a fresh deploy is the canonical reset signal — no
    /// dedicated `axhub reset` subcommand needed.
    pub fn can_transition_to(&self, next: &DeployPhase) -> bool {
        use DeployPhase::*;
        matches!(
            (self, next),
            (Idle, Preflight)
                | (Preflight, Resolve)
                | (Preflight, Failed)
                | (Resolve, Bootstrap)
                | (Resolve, Push)
                | (Resolve, Failed)
                | (Bootstrap, Push)
                | (Bootstrap, Failed)
                | (Push, Verify)
                | (Push, Failed)
                | (Verify, Completed)
                | (Verify, Failed)
                | (Failed, Preflight)
                | (Aborted, Preflight)
                | (Completed, Preflight)
        )
    }
}

/// Read the current phase from the event log and decide whether `next` is a
/// legal transition. Returns `Ok(())` when the move is valid, an `Err` with
/// the rejected pair otherwise. Errors are intentionally `String` because
/// callers surface them as Korean systemMessage text rather than
/// pattern-match on error types.
pub fn validate_transition(deploy_id: &str, next: DeployPhase) -> Result<(), String> {
    let current = current_phase(deploy_id)
        .and_then(|s| DeployPhase::parse(&s))
        .unwrap_or(DeployPhase::Idle);
    if current.can_transition_to(&next) {
        Ok(())
    } else {
        Err(format!(
            "invalid transition: {} → {}",
            current.as_str(),
            next.as_str()
        ))
    }
}

// =============================================================================
// Plan v6 Phase 0d — DiagnoseEvent schema (additive, backward compat with v1).
//
// The DeployEvent v1 schema remains the source of truth for deploy lifecycle.
// DiagnoseEvent is a parallel envelope for the auto-diagnose loop (Phase 1L
// → 5P). Both write NDJSON via `atomic_jsonl::append_line`, but to disjoint
// directories so readers don't need to discriminate at the file level:
//   <state>/deploy-events/{deploy_id}.jsonl    (existing)
//   <state>/diagnose-events/{loop_id}.jsonl    (new)
//
// Forward compatibility: `read_diagnose_events` silently skips lines whose
// `schema_version` does not match `DIAGNOSE_EVENT_SCHEMA_VERSION` so an older
// reader paired with a newer writer keeps working.
// =============================================================================

/// Pinned NDJSON envelope version for diagnose events.
pub const DIAGNOSE_EVENT_SCHEMA_VERSION: &str = "diagnose-event/v1";

/// One diagnose loop phase / state transition.
///
/// `phase` mirrors the plan v6 §3.1 5-Phase loop names plus internal
/// LOOP_VERIFY / ARCH_HANDOFF terminals. `evidence` is kind-specific
/// (e.g. for `hypothesis.selected` → rank + cause; for `loop_verify` →
/// pass/fail + duration_ms).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnoseEvent {
    pub schema_version: String,
    pub loop_id: String,
    /// ISO 8601 with millisecond precision, UTC ("Z" suffix).
    pub ts: String,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
}

impl DiagnoseEvent {
    /// Convenience constructor that stamps `schema_version` + ISO timestamp.
    pub fn new(loop_id: impl Into<String>, phase: impl Into<String>) -> Self {
        Self {
            schema_version: DIAGNOSE_EVENT_SCHEMA_VERSION.to_string(),
            loop_id: loop_id.into(),
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            phase: phase.into(),
            duration_ms: None,
            reason: None,
            evidence: None,
        }
    }
}

fn diagnose_events_dir() -> Option<PathBuf> {
    crate::runtime_paths::state_dir().map(|d| d.join("diagnose-events"))
}

fn diagnose_log_path(loop_id: &str) -> Result<PathBuf, EventLogError> {
    validate_deploy_id(loop_id)?; // shares the same single-segment rule
    let dir = diagnose_events_dir().ok_or(EventLogError::PathResolution)?;
    Ok(dir.join(format!("{loop_id}.jsonl")))
}

fn diagnose_disabled_via_env() -> bool {
    matches!(
        std::env::var("AXHUB_DISABLE_DIAGNOSE_EVENTS").as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

/// Append a diagnose event. Best-effort: returns `Ok(())` when the opt-out env
/// is set so callers can treat audit as advisory.
pub fn append_diagnose_event(loop_id: &str, event: &DiagnoseEvent) -> Result<(), EventLogError> {
    if diagnose_disabled_via_env() {
        return Ok(());
    }
    let path = diagnose_log_path(loop_id)?;
    let line = serde_json::to_string(event)?;
    atomic_jsonl::append_line(&path, &line)?;
    Ok(())
}

/// Read every diagnose event for a loop_id. Corrupt lines + unknown schema
/// rows are silently skipped (forward-compatible reader contract).
pub fn read_diagnose_events(loop_id: &str) -> Result<Vec<DiagnoseEvent>, EventLogError> {
    let path = diagnose_log_path(loop_id)?;
    let events = atomic_jsonl::read_lines(&path, |line| {
        let parsed: DiagnoseEvent = serde_json::from_str(line).ok()?;
        if parsed.schema_version == DIAGNOSE_EVENT_SCHEMA_VERSION {
            Some(parsed)
        } else {
            None
        }
    })?;
    Ok(events)
}

#[cfg(test)]
mod diagnose_tests {
    use super::*;

    #[test]
    fn diagnose_schema_version_pinned() {
        assert_eq!(DIAGNOSE_EVENT_SCHEMA_VERSION, "diagnose-event/v1");
    }

    #[test]
    fn diagnose_event_constructor_stamps_schema_and_ts() {
        let e = DiagnoseEvent::new("loop-001", "build_loop");
        assert_eq!(e.schema_version, DIAGNOSE_EVENT_SCHEMA_VERSION);
        assert_eq!(e.loop_id, "loop-001");
        assert_eq!(e.phase, "build_loop");
        // ISO 8601 millisecond UTC, ends with Z.
        assert!(e.ts.ends_with('Z'), "ts must be UTC: {}", e.ts);
    }

    #[test]
    fn diagnose_event_serde_roundtrip() {
        let mut e = DiagnoseEvent::new("loop-roundtrip", "fix.apply");
        e.duration_ms = Some(123);
        e.reason = Some("ok".into());
        e.evidence = Some(serde_json::json!({"hypothesis_rank": 1}));
        let line = serde_json::to_string(&e).unwrap();
        let parsed: DiagnoseEvent = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed, e);
    }

    #[test]
    fn diagnose_disabled_env_is_noop() {
        // Use a unique loop_id so we don't collide with other tests in parallel.
        let loop_id = "loop-disabled-noop-test";
        std::env::set_var("AXHUB_DISABLE_DIAGNOSE_EVENTS", "1");
        let e = DiagnoseEvent::new(loop_id, "should_not_write");
        let result = append_diagnose_event(loop_id, &e);
        std::env::remove_var("AXHUB_DISABLE_DIAGNOSE_EVENTS");
        assert!(result.is_ok(), "disabled env must be no-op success");
    }

    #[test]
    fn invalid_loop_id_rejected() {
        let e = DiagnoseEvent::new("bad", "phase");
        // Path traversal style id rejected by validate_deploy_id (..).
        let result = append_diagnose_event("..", &e);
        assert!(matches!(result, Err(EventLogError::InvalidDeployId(_))));
    }

    #[test]
    fn unknown_schema_version_silently_skipped_by_reader() {
        // This test only verifies the pure filter function; no fs touch needed.
        let mismatched = serde_json::json!({
            "schema_version": "diagnose-event/v999",
            "loop_id": "x",
            "ts": "2026-05-19T00:00:00.000Z",
            "phase": "p",
        })
        .to_string();
        let parsed: Option<DiagnoseEvent> = serde_json::from_str::<DiagnoseEvent>(&mismatched)
            .ok()
            .filter(|p| p.schema_version == DIAGNOSE_EVENT_SCHEMA_VERSION);
        assert!(parsed.is_none(), "reader must skip unknown schema");
    }
}

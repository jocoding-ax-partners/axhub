// Phase 25 PR 25.1 — idempotent recovery scan.
//
// Reads `$XDG_STATE_HOME/axhub-plugin/deploy-events/*.jsonl` (written by
// `event_log::append_event`, PR 26.1b) and surfaces any deploys whose last
// recorded phase is non-terminal — these are deploys that were interrupted
// (Ctrl-C, crash, lost terminal) and might still be in flight or, more
// commonly, abandoned.
//
// The caller (PR 25.1 follow-up that wires `main.rs` into the live deploy
// path) uses this to ask the user "resume / start fresh / abort" via
// AskUserQuestion before issuing a new `axhub deploy create`. Headless
// callers pick `fresh` by default per the registry contract.
//
// Plan reference:
// - .plan/matrix-absorption/phases/phase-25-tier-a-matrix-integration.md PR 25.1
// - Threshold default = 600 s (10 min) — anything older is considered
//   "abandoned" rather than "in-flight". Override via
//   `AXHUB_RECOVERY_THRESHOLD_SECS`.

use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::event_log::{self, DeployEvent, DeployPhase};

/// Default stale window. Deploys whose last event is older than this are
/// classified abandoned (not incomplete) regardless of phase.
pub const DEFAULT_STALE_THRESHOLD_SECS: u64 = 600;

/// Window passed to `event_log::list_recent_deploys`. Has to be ≥ the stale
/// threshold so we don't filter out the very deploys we want to evaluate.
pub const LIST_RECENT_WINDOW_SECS: u64 = 3_600;

/// Env override for the stale threshold. Hooked up to plan §R25-1 mitigation
/// ("data-driven default + env override").
pub const ENV_THRESHOLD: &str = "AXHUB_RECOVERY_THRESHOLD_SECS";

/// Disable recovery scan entirely (mirror of the PR 25.2 hook-safety env
/// taxonomy ADR §10.6). The caller site in `main.rs` will also short-circuit
/// via `hook_safety::is_hook_disabled("recovery-scan")` once that module
/// lands; this module-level check covers the helper subcommand path too.
const ENV_DISABLE: &str = "AXHUB_DISABLE_RECOVERY_SCAN";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryState {
    pub deploy_id: String,
    pub last_phase: String,
    pub last_ts: String,
    pub age_secs: u64,
    pub incomplete: bool,
}

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("event_log error: {0}")]
    EventLog(#[from] event_log::EventLogError),
    #[error("clock error: ts {0} could not be parsed as RFC3339")]
    ClockParse(String),
}

/// Resolve the effective stale threshold:
///   - `AXHUB_RECOVERY_THRESHOLD_SECS` env if set + parseable + non-zero
///   - otherwise `DEFAULT_STALE_THRESHOLD_SECS`
pub fn effective_threshold_secs() -> u64 {
    if let Ok(raw) = std::env::var(ENV_THRESHOLD) {
        if let Ok(parsed) = raw.parse::<u64>() {
            if parsed > 0 {
                return parsed;
            }
        }
    }
    DEFAULT_STALE_THRESHOLD_SECS
}

fn opted_out() -> bool {
    matches!(
        std::env::var(ENV_DISABLE).as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

/// Scan the deploy-events directory for deploys whose last recorded phase
/// is non-terminal AND whose age is within `threshold_secs`. Newest first.
pub fn detect_incomplete_deploys(threshold_secs: u64) -> Result<Vec<RecoveryState>, RecoveryError> {
    if opted_out() {
        return Ok(Vec::new());
    }
    let recent = event_log::list_recent_deploys(LIST_RECENT_WINDOW_SECS)?;
    let now = SystemTime::now();
    let mut states: Vec<RecoveryState> = Vec::new();

    for deploy_id in recent {
        let events = event_log::read_events(&deploy_id)?;
        let Some(state) = build_state(&deploy_id, &events, now)? else {
            continue;
        };
        if state.incomplete && state.age_secs <= threshold_secs {
            states.push(state);
        }
    }

    // Newest first (smallest age first). `list_recent_deploys` already
    // sorts by file mtime; we sort by event ts to be authoritative.
    states.sort_by_key(|state| state.age_secs);
    Ok(states)
}

/// Same scan as `detect_incomplete_deploys` but returns only the freshest
/// candidate. Used by `main.rs` to single-ask "resume this?" without
/// overwhelming the user with a list.
pub fn detect_most_recent_incomplete() -> Result<Option<RecoveryState>, RecoveryError> {
    let threshold = effective_threshold_secs();
    let states = detect_incomplete_deploys(threshold)?;
    Ok(states.into_iter().next())
}

fn build_state(
    deploy_id: &str,
    events: &[DeployEvent],
    now: SystemTime,
) -> Result<Option<RecoveryState>, RecoveryError> {
    let Some(last) = events.last() else {
        return Ok(None);
    };
    let last_phase_enum = DeployPhase::parse(&last.phase);
    let terminal = last_phase_enum.map(|p| p.is_terminal()).unwrap_or(false);
    let last_ts = parse_iso8601(&last.ts)?;
    let age_secs = now
        .duration_since(last_ts)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(Some(RecoveryState {
        deploy_id: deploy_id.to_string(),
        last_phase: last.phase.clone(),
        last_ts: last.ts.clone(),
        age_secs,
        incomplete: !terminal,
    }))
}

fn parse_iso8601(ts: &str) -> Result<SystemTime, RecoveryError> {
    let parsed: DateTime<Utc> = DateTime::parse_from_rfc3339(ts)
        .map_err(|_| RecoveryError::ClockParse(ts.to_string()))?
        .with_timezone(&Utc);
    let dur = parsed
        .signed_duration_since(DateTime::<Utc>::from_timestamp(0, 0).unwrap())
        .to_std()
        .map_err(|_| RecoveryError::ClockParse(ts.to_string()))?;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(dur.as_secs()))
}

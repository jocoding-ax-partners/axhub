// Phase 25 PR 25.1 — recovery_scan integration coverage.
//
// Mirrors `event_log_test.rs` env-isolation pattern: a process-wide mutex
// serializes XDG_STATE_HOME mutation so concurrent tests don't trample each
// other's deploy-events dirs.

use std::sync::Mutex;
use std::time::Duration;

use axhub_helpers::event_log::{self, DeployEvent};
use axhub_helpers::recovery_scan::{
    self, detect_incomplete_deploys, detect_most_recent_incomplete, effective_threshold_secs,
    DEFAULT_STALE_THRESHOLD_SECS,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    _state_home: tempfile::TempDir,
}

impl EnvGuard {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_STATE_HOME", dir.path());
        std::env::remove_var("AXHUB_DISABLE_DEPLOY_EVENTS");
        std::env::remove_var("AXHUB_DISABLE_RECOVERY_SCAN");
        std::env::remove_var("AXHUB_RECOVERY_THRESHOLD_SECS");
        Self { _state_home: dir }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var("XDG_STATE_HOME");
        std::env::remove_var("AXHUB_DISABLE_DEPLOY_EVENTS");
        std::env::remove_var("AXHUB_DISABLE_RECOVERY_SCAN");
        std::env::remove_var("AXHUB_RECOVERY_THRESHOLD_SECS");
    }
}

fn append_event_with_ts(deploy_id: &str, phase: &str, ts: chrono::DateTime<chrono::Utc>) {
    let event = DeployEvent {
        schema_version: event_log::EVENT_LOG_SCHEMA_VERSION.to_string(),
        deploy_id: deploy_id.to_string(),
        ts: ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        phase: phase.to_string(),
        exit_code: None,
        duration_ms: None,
        reason: None,
        evidence: None,
    };
    event_log::append_event(deploy_id, &event).unwrap();
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

#[test]
fn no_deploys_returns_empty_list() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let results = detect_incomplete_deploys(600).unwrap();
    assert!(results.is_empty());
    assert!(detect_most_recent_incomplete().unwrap().is_none());
}

#[test]
fn single_in_flight_deploy_within_window_is_flagged() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let two_min_ago = now() - chrono::Duration::seconds(120);
    append_event_with_ts("dep-fresh", "preflight", two_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].deploy_id, "dep-fresh");
    assert_eq!(results[0].last_phase, "preflight");
    assert!(results[0].incomplete);
    assert!(results[0].age_secs >= 110 && results[0].age_secs <= 130);
}

#[test]
fn deploy_older_than_threshold_is_excluded() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let twenty_min_ago = now() - chrono::Duration::seconds(20 * 60);
    append_event_with_ts("dep-stale", "preflight", twenty_min_ago);

    // Threshold = 600 (10 min). 20 min ago → excluded.
    let results = detect_incomplete_deploys(600).unwrap();
    assert!(
        results.is_empty(),
        "stale deploy must not appear, got: {results:?}"
    );
}

#[test]
fn deploy_ending_in_completed_is_not_flagged() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let two_min_ago = now() - chrono::Duration::seconds(120);
    append_event_with_ts("dep-done", "preflight", two_min_ago);
    append_event_with_ts("dep-done", "completed", two_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert!(
        results.is_empty(),
        "Completed-terminal deploy must not flag, got: {results:?}"
    );
}

#[test]
fn deploy_ending_in_failed_is_not_flagged() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let two_min_ago = now() - chrono::Duration::seconds(120);
    append_event_with_ts("dep-failed", "preflight", two_min_ago);
    append_event_with_ts("dep-failed", "failed", two_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert!(results.is_empty());
}

#[test]
fn multiple_in_flight_deploys_returned_newest_first() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let one_min_ago = now() - chrono::Duration::seconds(60);
    let four_min_ago = now() - chrono::Duration::seconds(240);
    append_event_with_ts("dep-older", "resolve", four_min_ago);
    append_event_with_ts("dep-newer", "preflight", one_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].deploy_id, "dep-newer", "newest first");
    assert_eq!(results[1].deploy_id, "dep-older");
}

#[test]
fn detect_most_recent_incomplete_returns_freshest() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let three_min_ago = now() - chrono::Duration::seconds(180);
    let one_min_ago = now() - chrono::Duration::seconds(60);
    append_event_with_ts("dep-old", "push", three_min_ago);
    append_event_with_ts("dep-new", "resolve", one_min_ago);

    let pick = detect_most_recent_incomplete().unwrap().unwrap();
    assert_eq!(pick.deploy_id, "dep-new");
    assert_eq!(pick.last_phase, "resolve");
}

#[test]
fn env_threshold_override_widens_window() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();
    std::env::set_var("AXHUB_RECOVERY_THRESHOLD_SECS", "1800"); // 30 min

    let fifteen_min_ago = now() - chrono::Duration::seconds(15 * 60);
    append_event_with_ts("dep-mid", "resolve", fifteen_min_ago);

    // default 600s would exclude; override 1800s includes.
    assert_eq!(effective_threshold_secs(), 1800);
    let pick = detect_most_recent_incomplete().unwrap();
    assert!(pick.is_some(), "15 min old should fit 30 min threshold");
}

#[test]
fn env_threshold_zero_falls_back_to_default() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();
    std::env::set_var("AXHUB_RECOVERY_THRESHOLD_SECS", "0");
    assert_eq!(effective_threshold_secs(), DEFAULT_STALE_THRESHOLD_SECS);
}

#[test]
fn env_threshold_garbage_falls_back_to_default() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();
    std::env::set_var("AXHUB_RECOVERY_THRESHOLD_SECS", "not-a-number");
    assert_eq!(effective_threshold_secs(), DEFAULT_STALE_THRESHOLD_SECS);
}

#[test]
fn opt_out_env_returns_empty_list_regardless_of_deploys() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();
    std::env::set_var("AXHUB_DISABLE_RECOVERY_SCAN", "1");

    let one_min_ago = now() - chrono::Duration::seconds(60);
    append_event_with_ts("dep-fresh", "preflight", one_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert!(results.is_empty(), "opt-out must skip scan entirely");
    assert!(detect_most_recent_incomplete().unwrap().is_none());
}

#[test]
fn unknown_terminal_string_falls_back_to_incomplete() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let two_min_ago = now() - chrono::Duration::seconds(120);
    // "frobnicate" is not a recognized DeployPhase → parse returns None →
    // treated as non-terminal (incomplete). Safer to flag than to silently
    // swallow a corrupted deploy.
    append_event_with_ts("dep-weird", "frobnicate", two_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].incomplete);
    assert_eq!(results[0].last_phase, "frobnicate");
}

#[test]
fn aborted_phase_is_terminal_so_skipped() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let two_min_ago = now() - chrono::Duration::seconds(120);
    append_event_with_ts("dep-aborted", "preflight", two_min_ago);
    append_event_with_ts("dep-aborted", "aborted", two_min_ago);

    let results = detect_incomplete_deploys(600).unwrap();
    assert!(results.is_empty(), "Aborted terminal must not flag");
}

#[test]
fn deploy_events_disabled_yields_empty_scan() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();
    std::env::set_var("AXHUB_DISABLE_DEPLOY_EVENTS", "1");
    // event_log::append_event becomes a no-op, so no logs exist.
    append_event_with_ts("dep-quiet", "preflight", now());
    let results = detect_incomplete_deploys(600).unwrap();
    assert!(results.is_empty());
    let _ = recovery_scan::DEFAULT_STALE_THRESHOLD_SECS; // touch const for coverage
    let _ = Duration::from_secs(0); // touch std::time::Duration import
}

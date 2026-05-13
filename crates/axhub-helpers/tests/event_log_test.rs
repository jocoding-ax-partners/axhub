// Phase 26 PR 26.1b — event_log integration coverage.
//
// Each test isolates `XDG_STATE_HOME` to a fresh tempdir so the
// `deploy-events/` directory it manipulates never collides with the host
// machine's real state. The opt-out env (`AXHUB_DISABLE_DEPLOY_EVENTS`) is
// also exercised because the plan promises silent no-op semantics there.

use std::fs;
use std::sync::Mutex;
use std::time::Duration;

use axhub_helpers::event_log::{
    self, current_phase, list_recent_deploys, read_events, DeployEvent, EVENT_LOG_SCHEMA_VERSION,
};

// Mutex serializes env-var mutation across the tests in this binary because
// `std::env::set_var` is process-global. Each test grabs the lock then
// rewires XDG_STATE_HOME to its own tempdir.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    _state_home: tempfile::TempDir,
}

impl EnvGuard {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_STATE_HOME", dir.path());
        std::env::remove_var("AXHUB_DISABLE_DEPLOY_EVENTS");
        Self { _state_home: dir }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var("XDG_STATE_HOME");
        std::env::remove_var("AXHUB_DISABLE_DEPLOY_EVENTS");
    }
}

fn make_event(deploy_id: &str, phase: &str) -> DeployEvent {
    DeployEvent::new(deploy_id, phase)
}

#[test]
fn write_one_event_then_read_one_event() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let event = make_event("dep-001", "preflight");
    event_log::append_event("dep-001", &event).unwrap();

    let events = read_events("dep-001").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].deploy_id, "dep-001");
    assert_eq!(events[0].phase, "preflight");
    assert_eq!(events[0].schema_version, EVENT_LOG_SCHEMA_VERSION);
}

#[test]
fn ten_events_preserve_order() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    for phase in [
        "preflight",
        "resolve",
        "bootstrap",
        "push",
        "verify",
        "completed",
    ] {
        event_log::append_event("dep-002", &make_event("dep-002", phase)).unwrap();
    }

    let events = read_events("dep-002").unwrap();
    let phases: Vec<&str> = events.iter().map(|e| e.phase.as_str()).collect();
    assert_eq!(
        phases,
        vec![
            "preflight",
            "resolve",
            "bootstrap",
            "push",
            "verify",
            "completed"
        ]
    );
}

#[test]
fn two_distinct_deploys_are_isolated() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    event_log::append_event("dep-A", &make_event("dep-A", "preflight")).unwrap();
    event_log::append_event("dep-B", &make_event("dep-B", "preflight")).unwrap();
    event_log::append_event("dep-A", &make_event("dep-A", "resolve")).unwrap();

    let a = read_events("dep-A").unwrap();
    let b = read_events("dep-B").unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 1);
    assert!(a.iter().all(|e| e.deploy_id == "dep-A"));
    assert!(b.iter().all(|e| e.deploy_id == "dep-B"));
}

#[test]
fn deploy_id_must_be_single_filename_segment() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let bad_ids = ["../probe", "nested/probe", r"nested\probe", "", ".", ".."];
    for deploy_id in bad_ids {
        let event = make_event(deploy_id, "preflight");
        assert!(
            event_log::append_event(deploy_id, &event).is_err(),
            "append_event should reject deploy_id={deploy_id:?}"
        );
        assert!(
            read_events(deploy_id).is_err(),
            "read_events should reject deploy_id={deploy_id:?}"
        );
        assert!(
            current_phase(deploy_id).is_none(),
            "current_phase should fail closed for deploy_id={deploy_id:?}"
        );
    }
}

#[test]
fn invalid_deploy_id_cannot_read_outside_deploy_events_dir() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let state_home = std::env::var("XDG_STATE_HOME").unwrap();
    let outside = std::path::Path::new(&state_home)
        .join("axhub-plugin")
        .join("probe.jsonl");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(
        &outside,
        r#"{"schema_version":"deploy-event/v1","deploy_id":"../probe","ts":"2026-05-11T00:00:00.000Z","phase":"failed","reason":"outside"}"#,
    )
    .unwrap();

    assert!(
        read_events("../probe").is_err(),
        "path traversal deploy_id must not read sibling JSONL files"
    );
}

#[test]
fn corrupt_line_in_the_middle_is_skipped_on_read() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    event_log::append_event("dep-corrupt", &make_event("dep-corrupt", "preflight")).unwrap();

    // Splice a non-JSON line into the file the same way audit corruption
    // might occur (e.g. disk full mid-write leaving a partial entry).
    let dir = axhub_helpers::runtime_paths::deploy_events_dir().unwrap();
    let path = dir.join("dep-corrupt.jsonl");
    {
        use std::io::Write;
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "{{not json at all").unwrap();
    }
    event_log::append_event("dep-corrupt", &make_event("dep-corrupt", "resolve")).unwrap();

    let events = read_events("dep-corrupt").unwrap();
    assert_eq!(events.len(), 2, "corrupt line must not block valid ones");
    assert_eq!(events[0].phase, "preflight");
    assert_eq!(events[1].phase, "resolve");
}

#[test]
fn unknown_schema_version_is_skipped() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    // Write a valid v1 entry, then plant a forward-incompatible v2 entry.
    event_log::append_event("dep-fwdcompat", &make_event("dep-fwdcompat", "preflight")).unwrap();

    let dir = axhub_helpers::runtime_paths::deploy_events_dir().unwrap();
    let path = dir.join("dep-fwdcompat.jsonl");
    {
        use std::io::Write;
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        let future = serde_json::json!({
            "schema_version": "deploy-event/v2",
            "deploy_id": "dep-fwdcompat",
            "ts": "2026-05-11T00:00:00.000Z",
            "phase": "resolve",
            "new_v2_field": true,
        });
        writeln!(f, "{future}").unwrap();
    }

    let events = read_events("dep-fwdcompat").unwrap();
    assert_eq!(events.len(), 1, "v2 entries must be skipped by v1 reader");
    assert_eq!(events[0].schema_version, EVENT_LOG_SCHEMA_VERSION);
}

#[test]
fn missing_directory_auto_created_on_first_append() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    let dir = axhub_helpers::runtime_paths::deploy_events_dir().unwrap();
    assert!(!dir.exists(), "deploy-events dir must not pre-exist");

    event_log::append_event("dep-new", &make_event("dep-new", "preflight")).unwrap();
    assert!(dir.exists());
    assert!(dir.join("dep-new.jsonl").exists());
}

#[test]
fn list_recent_deploys_filters_by_window() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    event_log::append_event("dep-fresh", &make_event("dep-fresh", "preflight")).unwrap();
    event_log::append_event("dep-stale", &make_event("dep-stale", "preflight")).unwrap();

    // Backdate dep-stale.jsonl to 1 hour ago via stable FileTimes API.
    let stale_path = axhub_helpers::runtime_paths::deploy_events_dir()
        .unwrap()
        .join("dep-stale.jsonl");
    let one_hour_ago = std::time::SystemTime::now() - Duration::from_secs(3600);
    let times = std::fs::FileTimes::new().set_modified(one_hour_ago);
    let f = fs::OpenOptions::new()
        .write(true)
        .open(&stale_path)
        .unwrap();
    f.set_times(times).unwrap();
    drop(f);

    // Window: last 5 minutes.
    let recent = list_recent_deploys(300).unwrap();
    assert_eq!(recent, vec!["dep-fresh".to_string()]);
}

#[test]
fn opt_out_env_makes_append_a_silent_no_op() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();
    std::env::set_var("AXHUB_DISABLE_DEPLOY_EVENTS", "1");

    event_log::append_event("dep-quiet", &make_event("dep-quiet", "preflight")).unwrap();

    let dir = axhub_helpers::runtime_paths::deploy_events_dir().unwrap();
    assert!(!dir.exists(), "opt-out must not create the directory");
    assert!(read_events("dep-quiet").unwrap().is_empty());
}

#[test]
fn current_phase_returns_last_event_phase() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    event_log::append_event("dep-cp", &make_event("dep-cp", "preflight")).unwrap();
    assert_eq!(current_phase("dep-cp").as_deref(), Some("preflight"));

    event_log::append_event("dep-cp", &make_event("dep-cp", "resolve")).unwrap();
    event_log::append_event("dep-cp", &make_event("dep-cp", "verify")).unwrap();
    assert_eq!(current_phase("dep-cp").as_deref(), Some("verify"));
}

#[test]
fn current_phase_for_unknown_deploy_returns_none() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    assert!(current_phase("never-existed").is_none());
}

// =============================================================================
// Phase 26 PR 26.2 — phase logic (DeployPhase enum + transition table).
//
// Table-driven coverage of the spec at .plan/matrix-absorption/phases/
// phase-26-tier-s-quick-wins.md PR 26.2 ("Transition rules" 표). Option (b)
// derived view: validate_transition reads `current_phase` from the event log
// so a fresh `Preflight` event after a terminal phase is a legal "implicit
// reset" — no `axhub reset` subcommand required.

use axhub_helpers::event_log::{validate_transition, DeployPhase};

#[test]
fn parse_and_as_str_round_trip_for_known_phases() {
    let names = [
        "idle",
        "preflight",
        "resolve",
        "bootstrap",
        "push",
        "verify",
        "completed",
        "failed",
        "aborted",
    ];
    for name in names {
        let phase = DeployPhase::parse(name).unwrap_or_else(|| panic!("parse failed: {name}"));
        assert_eq!(phase.as_str(), name);
    }
}

#[test]
fn parse_accepts_axhub_status_terminal_synonyms() {
    // statusline::TERMINAL_PHASES = ["complete", "succeeded", "failed", "cancelled", "errored"]
    // — verify the parser maps them onto the right enum variants so our
    // derived view stays compatible with raw axhub status output.
    assert_eq!(DeployPhase::parse("complete"), Some(DeployPhase::Completed));
    assert_eq!(
        DeployPhase::parse("succeeded"),
        Some(DeployPhase::Completed)
    );
    assert_eq!(DeployPhase::parse("success"), Some(DeployPhase::Completed));
    assert_eq!(DeployPhase::parse("errored"), Some(DeployPhase::Failed));
    assert_eq!(DeployPhase::parse("cancelled"), Some(DeployPhase::Aborted));
    assert_eq!(DeployPhase::parse("canceled"), Some(DeployPhase::Aborted));
}

#[test]
fn parse_unknown_phase_returns_none() {
    assert!(DeployPhase::parse("frobnicate").is_none());
    assert!(DeployPhase::parse("").is_none());
}

#[test]
fn is_terminal_only_for_completed_failed_aborted() {
    use DeployPhase::*;
    for phase in [Completed, Failed, Aborted] {
        assert!(phase.is_terminal(), "{} should be terminal", phase.as_str());
    }
    for phase in [Idle, Preflight, Resolve, Bootstrap, Push, Verify] {
        assert!(
            !phase.is_terminal(),
            "{} should NOT be terminal",
            phase.as_str()
        );
    }
}

#[test]
fn happy_path_transitions_are_legal() {
    use DeployPhase::*;
    let happy_path = [
        (Idle, Preflight),
        (Preflight, Resolve),
        (Resolve, Bootstrap),
        (Bootstrap, Push),
        (Push, Verify),
        (Verify, Completed),
    ];
    for (from, to) in happy_path {
        assert!(
            from.can_transition_to(&to),
            "{} → {} must be legal",
            from.as_str(),
            to.as_str()
        );
    }
}

#[test]
fn resolve_can_short_circuit_directly_to_push() {
    // Cached bootstrap (e.g. apphub.yaml unchanged) skips Bootstrap.
    assert!(DeployPhase::Resolve.can_transition_to(&DeployPhase::Push));
}

#[test]
fn any_non_terminal_phase_can_jump_to_failed() {
    use DeployPhase::*;
    for from in [Preflight, Resolve, Bootstrap, Push, Verify] {
        assert!(
            from.can_transition_to(&Failed),
            "{} → failed must be legal (deploy can fail at any phase)",
            from.as_str()
        );
    }
}

#[test]
fn terminal_phases_can_implicit_reset_to_preflight() {
    use DeployPhase::*;
    // Last-event-wins: a fresh Preflight after a terminal phase is the
    // implicit reset. No `axhub reset` subcommand required.
    assert!(Failed.can_transition_to(&Preflight));
    assert!(Aborted.can_transition_to(&Preflight));
    assert!(Completed.can_transition_to(&Preflight));
}

#[test]
fn invalid_transitions_are_rejected() {
    use DeployPhase::*;
    let illegal = [
        (Idle, Push), // must do Preflight first
        (Idle, Verify),
        (Preflight, Verify),    // must Resolve first
        (Preflight, Completed), // can't complete before push
        (Bootstrap, Verify),    // must Push first
        (Verify, Idle),         // terminal-to-Idle banned
        (Completed, Verify),    // can't rewind from terminal
        (Failed, Verify),
        (Aborted, Push),
    ];
    for (from, to) in illegal {
        assert!(
            !from.can_transition_to(&to),
            "{} → {} must be rejected",
            from.as_str(),
            to.as_str()
        );
    }
}

#[test]
fn validate_transition_uses_current_phase_from_event_log() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    // No events yet → current_phase = None → defaults to Idle. Idle → Preflight legal.
    assert!(validate_transition("dep-vt", DeployPhase::Preflight).is_ok());

    // Drop a preflight event. current_phase becomes "preflight".
    event_log::append_event("dep-vt", &make_event("dep-vt", "preflight")).unwrap();
    assert!(validate_transition("dep-vt", DeployPhase::Resolve).is_ok());
    let err = validate_transition("dep-vt", DeployPhase::Completed).unwrap_err();
    assert!(err.contains("preflight"));
    assert!(err.contains("completed"));
}

#[test]
fn validate_transition_after_terminal_allows_preflight_only() {
    let _g = ENV_LOCK.lock().unwrap();
    let _env = EnvGuard::new();

    for phase in ["preflight", "resolve", "push", "verify", "completed"] {
        event_log::append_event("dep-after-term", &make_event("dep-after-term", phase)).unwrap();
    }
    assert_eq!(
        current_phase("dep-after-term").as_deref(),
        Some("completed"),
        "current_phase derives from last event"
    );
    // Implicit reset: Completed → Preflight is allowed.
    assert!(validate_transition("dep-after-term", DeployPhase::Preflight).is_ok());
    // But Completed → Verify is rejected (no rewinding).
    assert!(validate_transition("dep-after-term", DeployPhase::Verify).is_err());
}

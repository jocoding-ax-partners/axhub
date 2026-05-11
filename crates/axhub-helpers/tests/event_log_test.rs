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

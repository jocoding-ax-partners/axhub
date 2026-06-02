// Phase 25 PR 25.4 — integration coverage for trace_helper::trace.
//
// Kept as an integration test instead of an in-module unit test because it
// mutates XDG_STATE_HOME to isolate event-log IO. Running it in a separate test
// binary avoids racing telemetry unit tests that also isolate runtime paths.

use std::sync::{Mutex, OnceLock};

use axhub_helpers::event_log::{self, DeployEvent};
use axhub_helpers::trace_helper::{self, RoutingContext, TraceProbes};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    _state_home: tempfile::TempDir,
    old_xdg_state_home: Option<String>,
    old_disable_deploy_events: Option<String>,
}

impl EnvGuard {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let old_xdg_state_home = std::env::var("XDG_STATE_HOME").ok();
        let old_disable_deploy_events = std::env::var("AXHUB_DISABLE_DEPLOY_EVENTS").ok();
        std::env::set_var("XDG_STATE_HOME", dir.path());
        std::env::remove_var("AXHUB_DISABLE_DEPLOY_EVENTS");
        Self {
            _state_home: dir,
            old_xdg_state_home,
            old_disable_deploy_events,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old_xdg_state_home {
            Some(value) => std::env::set_var("XDG_STATE_HOME", value),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
        match &self.old_disable_deploy_events {
            Some(value) => std::env::set_var("AXHUB_DISABLE_DEPLOY_EVENTS", value),
            None => std::env::remove_var("AXHUB_DISABLE_DEPLOY_EVENTS"),
        }
    }
}

struct FakeProbes {
    build_log: String,
    routing: Option<RoutingContext>,
}

impl TraceProbes for FakeProbes {
    fn axhub_build_log(&self, _deploy_id: &str, _tail: u32) -> String {
        self.build_log.clone()
    }

    fn recent_routing_context(&self) -> Option<RoutingContext> {
        self.routing.clone()
    }
}

fn make_event(deploy_id: &str, phase: &str, duration_ms: Option<u64>) -> DeployEvent {
    let mut event = DeployEvent::new(deploy_id, phase);
    event.duration_ms = duration_ms;
    event
}

#[test]
fn trace_reads_event_log_and_merges_probe_evidence() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _env = EnvGuard::new();
    let deploy_id = "dep-trace-merge";
    event_log::append_event(deploy_id, &make_event(deploy_id, "preflight", Some(120))).unwrap();
    event_log::append_event(deploy_id, &make_event(deploy_id, "push", Some(250))).unwrap();
    let mut failed = make_event(deploy_id, "failed", None);
    failed.reason = Some("build command failed".to_string());
    event_log::append_event(deploy_id, &failed).unwrap();

    let probes = FakeProbes {
        build_log:
            "INFO ok\nERROR cannot find module 'vite'\nWARN network timeout while fetching\n"
                .to_string(),
        routing: Some(RoutingContext {
            last_routing_audit_ts: "2026-05-11T00:00:00Z".to_string(),
            last_prompt_hash_prefix: "abcdef123456".to_string(),
            is_axhub_related_recent: true,
        }),
    };

    let report = trace_helper::trace(deploy_id, &probes).unwrap();
    assert_eq!(report.deploy_id, deploy_id);
    assert_eq!(report.last_phase, "failed");
    assert_eq!(
        report.failure_reason.as_deref(),
        Some("build command failed")
    );
    assert_eq!(report.phase_durations.len(), 3);
    assert_eq!(report.phase_durations[1].phase, "push");
    assert_eq!(report.build_log_errors.len(), 2);
    // failure_reason ("build command failed") is always matched first
    // (authoritative), then the severity-gated runtime-log lines.
    assert_eq!(
        report.matched_patterns,
        vec![
            "build_command_failed".to_string(),
            "module_not_found".to_string(),
            "network_timeout".to_string()
        ]
    );
    assert_eq!(
        report
            .routing_context
            .as_ref()
            .map(|r| r.last_prompt_hash_prefix.as_str()),
        Some("abcdef123456")
    );
}

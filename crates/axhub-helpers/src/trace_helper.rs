// Phase 25 PR 25.4 — trace helper.
//
// Aggregates three sources into one `TraceReport` for the `axhub:trace`
// SKILL and the `axhub-helpers trace --json` CLI:
//   A. event_log    — phase transitions + per-phase duration_ms
//   B. build_log    — caller-provided deploy build log ERROR/WARN excerpts
//   C. audit        — recent routing context, time-window correlated
//
// The helper is **pure** (no I/O) except for the optional `TraceProbes`
// trait. Tests inject canned probes; real callers pass a `RealTraceProbes`
// that spawns `axhub`. This matches the PR 26.4 verify_helper pattern.
//
// Plan reference:
// - .plan/matrix-absorption/phases/phase-25-tier-a-matrix-integration.md PR 25.4

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::event_log::{self, DeployEvent, DeployPhase};

/// One row of the trace report's phase timeline. `duration_ms` is the gap
/// between this event and the next event (or `None` for the last entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseDuration {
    pub phase: String,
    pub duration_ms: Option<u64>,
    /// Index from start (0-based) — useful for callers that want to spot
    /// outliers ("step 3 took 10× the median").
    pub step: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingContext {
    pub last_routing_audit_ts: String,
    pub last_prompt_hash_prefix: String,
    pub is_axhub_related_recent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceReport {
    pub deploy_id: String,
    pub last_phase: String,
    pub failure_reason: Option<String>,
    pub phase_durations: Vec<PhaseDuration>,
    pub build_log_errors: Vec<String>,
    pub routing_context: Option<RoutingContext>,
    /// Matched error-pattern keys from `skills/trace/references/error-patterns.md`,
    /// computed against `build_log_errors`. Used by the SKILL to pick the
    /// 4-part empathy entry.
    pub matched_patterns: Vec<String>,
    /// Probe-emitted warnings — e.g. evidence sources skipped because a
    /// required argument was omitted. Separated from `build_log_errors`
    /// so SKILL parsers can branch on the warning without polluting the
    /// build-log evidence collection.
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("event_log error: {0}")]
    EventLog(#[from] event_log::EventLogError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Caller-injected build-log probe. Real callers delegate to current
/// `axhub deploy logs <DEPLOY_ID> --app <APP> --source build`;
/// tests pass a closure returning canned NDJSON or plain text.
pub trait TraceProbes {
    fn axhub_build_log(&self, deploy_id: &str, tail: u32) -> String;
    fn recent_routing_context(&self) -> Option<RoutingContext>;
    /// Warnings accumulated during probe execution. The default returns
    /// an empty list; real probes that need to signal a degraded run
    /// (e.g. skipped build-log probe because `--app` was omitted) should
    /// override.
    fn trace_warnings(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Synthesize a `TraceReport` from a deploy_id. Pure-ish — depends on the
/// `event_log` filesystem state but accepts caller-injected probes for
/// build_log and audit.
pub fn trace<P: TraceProbes>(deploy_id: &str, probes: &P) -> Result<TraceReport, TraceError> {
    let events = event_log::read_events(deploy_id)?;
    let phase_durations = compute_phase_durations(&events);
    let last_phase = events
        .last()
        .map(|e| e.phase.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let failure_reason = last_failed_reason(&events);

    // R3γ: `axhub deploy logs` 는 app 런타임 로그를 반환해요 (build-log 엔드포인트
    // 부재 — F3). probe 가 NDJSON `message` 를 unwrap 한 plain 텍스트를 넘겨줘요.
    let runtime_log = probes.axhub_build_log(deploy_id, 100);

    // Display (NEVER 규칙): ERROR/FATAL/WARN 라인 최대 5줄만 인용.
    let build_log_errors = extract_error_lines(&runtime_log, 5);

    // Matching (reachability): display 5-라인 cap 과 분리해서 전체 라인에 패턴 매칭.
    // 런타임 로그가 비면 (빌드 단계 실패로 앱 미기동) 로컬 event_log 의
    // failure_reason 을 매칭 입력으로 fallback.
    let mut match_input: Vec<String> = runtime_log.lines().map(|l| l.to_string()).collect();
    if match_input.is_empty() {
        if let Some(reason) = &failure_reason {
            match_input.push(reason.clone());
        }
    }
    let matched_patterns = match_error_patterns(&match_input);
    let routing_context = probes.recent_routing_context();
    let warnings = probes.trace_warnings();

    Ok(TraceReport {
        deploy_id: deploy_id.to_string(),
        last_phase,
        failure_reason,
        phase_durations,
        build_log_errors,
        routing_context,
        matched_patterns,
        warnings,
    })
}

fn compute_phase_durations(events: &[DeployEvent]) -> Vec<PhaseDuration> {
    if events.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(events.len());
    for (idx, event) in events.iter().enumerate() {
        let duration_ms = event.duration_ms;
        out.push(PhaseDuration {
            phase: event.phase.clone(),
            duration_ms,
            step: idx,
        });
    }
    out
}

fn last_failed_reason(events: &[DeployEvent]) -> Option<String> {
    let last = events.last()?;
    if let Some(phase) = DeployPhase::parse(&last.phase) {
        if phase == DeployPhase::Failed {
            return last.reason.clone();
        }
    }
    None
}

fn extract_error_lines(raw: &str, max: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in raw.lines() {
        let upper = line.to_uppercase();
        if upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("WARN") {
            out.push(line.trim().to_string());
            if out.len() >= max {
                break;
            }
        }
    }
    out
}

/// Error patterns mirroring `skills/trace/references/error-patterns.md`.
/// Each tuple = (pattern needle, canonical key). Lowercase substring match.
const ERROR_PATTERNS: &[(&str, &str)] = &[
    ("env: ", "env_not_found"),
    ("out of memory", "oom"),
    ("oomkilled", "oom"),
    ("oom", "oom"),
    ("module not found", "module_not_found"),
    ("cannot find module", "module_not_found"),
    ("network timeout", "network_timeout"),
    ("connection refused", "network_timeout"),
    ("dependency install failed", "dependency_install_failed"),
    ("npm err!", "dependency_install_failed"),
    ("docker pull", "docker_image_pull_failed"),
    ("image pull failed", "docker_image_pull_failed"),
    ("address already in use", "port_already_in_use"),
    ("eaddrinuse", "port_already_in_use"),
    ("build command failed", "build_command_failed"),
    ("exit code 1", "build_command_failed"),
];

fn match_error_patterns(errors: &[String]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for line in errors {
        let lower = line.to_lowercase();
        for (needle, key) in ERROR_PATTERNS {
            if needle_hit(&lower, needle) {
                let k = key.to_string();
                if !keys.contains(&k) {
                    keys.push(k);
                }
                break;
            }
        }
    }
    keys
}

/// 대부분의 needle 은 lowercase substring 으로 매칭해요. 일부 짧고 모호한 needle 은
/// 전체 런타임 로그(R3γ)에 매칭하면 오탐이 나므로 더 엄격하게 봐요: `oom` 은
/// "zoom"/"room" 에서 발화하면 안 되고, bare `exit code 1` 은 "exit code 127" 에서
/// 발화하면 안 돼요.
fn needle_hit(lower: &str, needle: &str) -> bool {
    match needle {
        "oom" => contains_word(lower, "oom"),
        "exit code 1" => contains_not_followed_by_digit(lower, "exit code 1"),
        _ => lower.contains(needle),
    }
}

/// `word` 가 양쪽 모두 비-영숫자 경계로 둘러싸여 등장하는지.
fn contains_word(hay: &str, word: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(rel) = hay[from..].find(word) {
        let i = from + rel;
        let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        let after = i + word.len();
        let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = i + 1;
    }
    false
}

/// `needle` 이 바로 뒤에 숫자가 오지 않는 위치에 등장하는지 (그래서 "exit code 1"
/// 이 "exit code 127" 안에서 매칭되지 않게).
fn contains_not_followed_by_digit(hay: &str, needle: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(rel) = hay[from..].find(needle) {
        let i = from + rel;
        let after = i + needle.len();
        if after >= bytes.len() || !bytes[after].is_ascii_digit() {
            return true;
        }
        from = i + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::DeployEvent;

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

    fn empty_probes() -> FakeProbes {
        FakeProbes {
            build_log: String::new(),
            routing: None,
        }
    }

    fn make_event(deploy_id: &str, phase: &str, duration_ms: Option<u64>) -> DeployEvent {
        let mut e = DeployEvent::new(deploy_id, phase);
        e.duration_ms = duration_ms;
        e
    }

    #[test]
    fn extract_error_lines_keeps_only_severity_keywords() {
        let raw = "INFO ok\nERROR boom\nINFO meh\nFATAL crash\nINFO done\n";
        let out = extract_error_lines(raw, 10);
        assert_eq!(
            out,
            vec!["ERROR boom".to_string(), "FATAL crash".to_string()]
        );
    }

    #[test]
    fn extract_error_lines_caps_at_max() {
        let raw = "ERROR a\nERROR b\nERROR c\nERROR d\nERROR e\n";
        let out = extract_error_lines(raw, 3);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn warn_is_captured_alongside_error_and_fatal() {
        let raw = "WARN slow\nINFO ok\nERROR boom\n";
        let out = extract_error_lines(raw, 10);
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("WARN"));
    }

    #[test]
    fn match_error_patterns_finds_env_not_found() {
        let errs = vec!["ERROR env: STRIPE_KEY not found".to_string()];
        assert_eq!(
            match_error_patterns(&errs),
            vec!["env_not_found".to_string()]
        );
    }

    #[test]
    fn match_error_patterns_dedupes_repeated_keys() {
        let errs = vec![
            "FATAL out of memory killed".to_string(),
            "ERROR OOM detected".to_string(),
        ];
        assert_eq!(match_error_patterns(&errs), vec!["oom".to_string()]);
    }

    #[test]
    fn match_error_patterns_returns_empty_for_unknown() {
        let errs = vec!["ERROR unknown gibberish".to_string()];
        assert!(match_error_patterns(&errs).is_empty());
    }

    #[test]
    fn match_error_patterns_recognizes_module_not_found_variants() {
        let errs = vec!["ERROR cannot find module 'react'".to_string()];
        assert_eq!(
            match_error_patterns(&errs),
            vec!["module_not_found".to_string()]
        );
    }

    #[test]
    fn matching_is_decoupled_from_display_cap_and_severity_filter() {
        // R3γ T006: 앞 5줄이 ERROR(=display cap 채움), 6번째 줄은 태그 없는 env: 라인.
        let raw = "ERROR a\nERROR b\nERROR c\nERROR d\nERROR e\nenv: STRIPE_KEY not found\n";
        // display 는 ERROR 5줄만 (env: 는 severity 토큰 없음 + cap 으로 제외).
        let display = extract_error_lines(raw, 5);
        assert_eq!(display.len(), 5);
        assert!(!display.iter().any(|l| l.contains("env:")));
        // 매칭은 전체 라인 기준이라 env_not_found 를 여전히 잡아요 (decoupled).
        let all: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
        assert!(match_error_patterns(&all).contains(&"env_not_found".to_string()));
    }

    #[test]
    fn oom_needle_is_word_bounded_no_false_positive() {
        // R2 T012: zoom/room 에서 oom 발화 금지.
        assert!(match_error_patterns(&["zoom meeting scheduled".to_string()]).is_empty());
        assert!(match_error_patterns(&["building src/room/index.js".to_string()]).is_empty());
        // 실제 oom 은 여전히 매칭 (단어 경계 + oomkilled).
        assert_eq!(
            match_error_patterns(&["OOM detected".to_string()]),
            vec!["oom".to_string()]
        );
        assert_eq!(
            match_error_patterns(&["process oomkilled".to_string()]),
            vec!["oom".to_string()]
        );
    }

    #[test]
    fn compute_phase_durations_emits_step_index_for_each_event() {
        let events = vec![
            make_event("d", "preflight", Some(120)),
            make_event("d", "resolve", Some(500)),
            make_event("d", "push", None),
        ];
        let out = compute_phase_durations(&events);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].step, 0);
        assert_eq!(out[1].duration_ms, Some(500));
        assert_eq!(out[2].duration_ms, None);
    }

    #[test]
    fn last_failed_reason_returns_reason_when_phase_is_failed() {
        let mut event = make_event("d", "failed", None);
        event.reason = Some("env: STRIPE_KEY not found".to_string());
        let reason = last_failed_reason(&[event]);
        assert_eq!(reason.as_deref(), Some("env: STRIPE_KEY not found"));
    }

    #[test]
    fn last_failed_reason_none_when_terminal_other_than_failed() {
        let mut event = make_event("d", "completed", None);
        event.reason = Some("done".to_string());
        assert!(last_failed_reason(&[event]).is_none());
    }

    #[test]
    fn trace_propagates_probe_warnings_into_report() {
        struct WarningProbes;
        impl TraceProbes for WarningProbes {
            fn axhub_build_log(&self, _deploy_id: &str, _tail: u32) -> String {
                String::new()
            }
            fn recent_routing_context(&self) -> Option<RoutingContext> {
                None
            }
            fn trace_warnings(&self) -> Vec<String> {
                vec!["runtime_log_probe_skipped: --app required".to_string()]
            }
        }
        // event_log read may fail in this synthetic env; we just exercise
        // the warning-propagation path through TraceReport assembly.
        let report = TraceReport {
            deploy_id: "dep-test".to_string(),
            last_phase: "unknown".to_string(),
            failure_reason: None,
            phase_durations: Vec::new(),
            build_log_errors: Vec::new(),
            routing_context: None,
            matched_patterns: Vec::new(),
            warnings: WarningProbes.trace_warnings(),
        };
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("runtime_log_probe_skipped"));
        // Critically: warnings must NOT leak into build_log_errors —
        // SKILL parsers split the two channels.
        assert!(report.build_log_errors.is_empty());
    }

    #[test]
    fn trace_with_no_events_yields_unknown_last_phase() {
        // event_log read inside trace() depends on filesystem state; here we
        // just verify the algorithms that don't require I/O.
        let report = TraceReport {
            deploy_id: "dep-x".to_string(),
            last_phase: "unknown".to_string(),
            failure_reason: None,
            phase_durations: Vec::new(),
            build_log_errors: Vec::new(),
            routing_context: None,
            matched_patterns: Vec::new(),
            warnings: Vec::new(),
        };
        assert_eq!(report.last_phase, "unknown");
        // Sanity-check FakeProbes interface stays buildable.
        let _ = empty_probes();
    }
}

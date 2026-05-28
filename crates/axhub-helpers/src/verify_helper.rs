// Phase 26 PR 26.4 — verify helper.
//
// Wraps current `axhub deploy list/status/logs` probes for the
// `axhub:verify` SKILL. Designed for both JSON-pipe consumers (CI scripts,
// `axhub-helpers verify --json`) and the human-facing Korean verdict the
// SKILL prints. Each external call is bounded at 5 s so an unreachable
// hub-api never hangs the verifier.
//
// Returns a `VerifyResult` describing one of three verdicts:
//   - `Live`    — state=live + recent deploy + 0 ERROR/FATAL log lines
//   - `Suspect` — at least one anomaly but no terminal failure
//   - `NotLive` — state ≠ live OR no recent deploy
//
// The helper is pure (no I/O) for everything except `axhub_cli_runner` —
// callers in tests inject a fake runner so the suite stays hermetic.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Live,
    Suspect,
    NotLive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyResult {
    pub verdict: Verdict,
    pub state: Option<String>,
    pub last_deploy_id: Option<String>,
    pub last_deploy_age_secs: Option<u64>,
    pub errors: Vec<String>,
    /// Plain-language Korean reasons surfacing why the verdict was chosen.
    /// SKILL prints these verbatim under the verdict line.
    pub reasons: Vec<String>,
}

/// Caller-injected probes. Both return tuple of (stdout, exit_code). The
/// trait is intentionally small so a test can pass a closure returning
/// canned responses without setting up `axhub` on PATH.
pub trait VerifyProbes {
    fn axhub_status(&self, app_id: &str) -> ProbeResult;
    fn axhub_logs_tail(&self, app_id: &str, lines: u32) -> ProbeResult;
}

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub stdout: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Sentinel `stdout` body emitted by [`ProbeResult::no_recent_deploy`].
/// `run_verify` matches against this constant *before* attempting a
/// `serde_json::from_str` so the typed "no recent deploy" outcome
/// short-circuits straight to a NotLive verdict — eliminating the
/// brittle JSON literal roundtrip review #18 flagged.
pub(crate) const NO_RECENT_DEPLOY_STDOUT: &str = r#"{"state":"unknown","last_deploy_id":null}"#;

impl ProbeResult {
    /// "App has no recent deploys" outcome. Used by `axhub_status` /
    /// `axhub_logs_tail` when `latest_deploy_id_for_app` returns nothing
    /// — distinct from a transport / auth failure (which should propagate
    /// the underlying exit code). PR #149 / review #18 — single source of
    /// truth for the sentinel `stdout` that `run_verify` recognises.
    pub fn no_recent_deploy() -> Self {
        Self {
            stdout: NO_RECENT_DEPLOY_STDOUT.to_string(),
            exit_code: 0,
            timed_out: false,
        }
    }
}

/// Live state synonyms accepted from axhub status (`state` field).
const LIVE_STATES: &[&str] = &["live", "running", "deployed", "active", "ok", "succeeded"];

const ERROR_PATTERNS: &[&str] = &["ERROR", "FATAL"];

/// Maximum age (in seconds) of "last deploy" that still counts as "fresh
/// enough to verify". Older than this and the verifier degrades to NotLive
/// with a "no recent deploy" reason.
const FRESH_DEPLOY_WINDOW_SECS: u64 = 600;

pub fn run_verify<P: VerifyProbes>(app_id: &str, probes: &P) -> VerifyResult {
    let status = probes.axhub_status(app_id);
    let logs = probes.axhub_logs_tail(app_id, 50);

    let mut reasons: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut state: Option<String> = None;
    let mut last_deploy_id: Option<String> = None;
    let mut last_deploy_age_secs: Option<u64> = None;

    let mut status_observed = false;
    if status.timed_out {
        reasons.push("axhub status timeout (5초)".to_string());
    } else if status.exit_code != 0 {
        reasons.push(format!("axhub status exit code {}", status.exit_code));
    } else if status.stdout == NO_RECENT_DEPLOY_STDOUT {
        // Typed short-circuit — caller signalled "no recent deploy" via
        // ProbeResult::no_recent_deploy() instead of going through the
        // serde_json roundtrip. Mark status_observed so the downstream
        // verdict logic still computes (will fall through to NotLive via
        // missing state_live + the explicit "최근 배포 없음" reason).
        status_observed = true;
        reasons.push("최근 배포 없음".to_string());
    } else if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&status.stdout) {
        status_observed = true;
        let data = if parsed.get("schema_version").is_some() || parsed.get("status").is_some() {
            parsed.get("data").unwrap_or(&parsed)
        } else {
            &parsed
        };
        // Transport-failure escape hatch (PR #149 / review #8): when the
        // helper synthesised the status because deploy-list failed, surface
        // the specific reason instead of letting "state = transport_error"
        // be the only signal.
        if let Some(reason) = data.get("transport_reason").and_then(|v| v.as_str()) {
            reasons.push(reason.to_string());
        }
        state = data
            .get("state")
            .or_else(|| data.get("status"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        last_deploy_id = data
            .get("last_deploy_id")
            .or_else(|| data.get("deployment_id"))
            .or_else(|| data.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        last_deploy_age_secs = data
            .get("last_deploy_age_secs")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                data.get("started_at")
                    .or_else(|| data.get("created_at"))
                    .and_then(|v| v.as_str())
                    .and_then(age_secs_from_rfc3339)
            });
    } else {
        reasons.push("axhub status JSON parse 실패".to_string());
    }

    let state_live = state
        .as_deref()
        .map(|s| LIVE_STATES.iter().any(|live| live.eq_ignore_ascii_case(s)))
        .unwrap_or(false);
    if !state_live {
        if let Some(s) = state.as_deref() {
            reasons.push(format!("state = \"{s}\" (live 아님)"));
        } else {
            reasons.push("state 필드 부재".to_string());
        }
    }

    if status_observed {
        match last_deploy_age_secs {
            Some(age) if age > FRESH_DEPLOY_WINDOW_SECS => {
                reasons.push(format!("최근 배포 없음 ({age}초 전)"));
            }
            Some(_) => {}
            None => reasons.push("최근 배포 없음".to_string()),
        }
    }

    if logs.timed_out {
        reasons.push("axhub logs timeout (5초)".to_string());
    } else if logs.exit_code != 0 {
        reasons.push(format!("axhub logs exit code {}", logs.exit_code));
    } else {
        for line in logs.stdout.lines() {
            if ERROR_PATTERNS.iter().any(|p| line.contains(p)) {
                errors.push(line.to_string());
                if errors.len() >= 3 {
                    break;
                }
            }
        }
        if !errors.is_empty() {
            reasons.push(format!("runtime ERROR/FATAL {} 건", errors.len()));
        }
    }

    let verdict = compute_verdict(
        status_observed,
        state_live,
        last_deploy_age_secs,
        &errors,
        &reasons,
    );

    VerifyResult {
        verdict,
        state,
        last_deploy_id,
        last_deploy_age_secs,
        errors,
        reasons,
    }
}

fn age_secs_from_rfc3339(raw: &str) -> Option<u64> {
    let ts = chrono::DateTime::parse_from_rfc3339(raw)
        .ok()?
        .with_timezone(&chrono::Utc);
    let now = chrono::Utc::now();
    Some(now.timestamp().saturating_sub(ts.timestamp()).max(0) as u64)
}

fn compute_verdict(
    status_observed: bool,
    state_live: bool,
    age_secs: Option<u64>,
    errors: &[String],
    reasons: &[String],
) -> Verdict {
    if !status_observed {
        return Verdict::Suspect;
    }
    let fresh = matches!(age_secs, Some(age) if age <= FRESH_DEPLOY_WINDOW_SECS);
    if state_live && fresh && errors.is_empty() && reasons.is_empty() {
        Verdict::Live
    } else if !state_live || !fresh {
        // state ≠ live OR no fresh deploy — emit NotLive so SKILL routes to trace.
        Verdict::NotLive
    } else {
        // state=live + fresh deploy but with at least one anomaly (runtime
        // errors / logs probe failure) → Suspect.
        Verdict::Suspect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProbes {
        status_stdout: String,
        status_exit: i32,
        logs_stdout: String,
        logs_exit: i32,
    }

    impl VerifyProbes for FakeProbes {
        fn axhub_status(&self, _app_id: &str) -> ProbeResult {
            ProbeResult {
                stdout: self.status_stdout.clone(),
                exit_code: self.status_exit,
                timed_out: false,
            }
        }
        fn axhub_logs_tail(&self, _app_id: &str, _lines: u32) -> ProbeResult {
            ProbeResult {
                stdout: self.logs_stdout.clone(),
                exit_code: self.logs_exit,
                timed_out: false,
            }
        }
    }

    fn happy_probes() -> FakeProbes {
        FakeProbes {
            status_stdout:
                r#"{"state":"live","last_deploy_id":"dep-abc","last_deploy_age_secs":120}"#
                    .to_string(),
            status_exit: 0,
            logs_stdout: "INFO server up\nINFO request handled\n".to_string(),
            logs_exit: 0,
        }
    }

    #[test]
    fn happy_path_returns_live_verdict() {
        let result = run_verify("paydrop", &happy_probes());
        assert_eq!(result.verdict, Verdict::Live);
        assert_eq!(result.state.as_deref(), Some("live"));
        assert_eq!(result.last_deploy_id.as_deref(), Some("dep-abc"));
        assert_eq!(result.last_deploy_age_secs, Some(120));
        assert!(result.errors.is_empty());
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn state_not_live_returns_not_live_verdict() {
        let probes = FakeProbes {
            status_stdout:
                r#"{"state":"rolled_back","last_deploy_id":"dep-1","last_deploy_age_secs":60}"#
                    .to_string(),
            status_exit: 0,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert_eq!(result.verdict, Verdict::NotLive);
        assert!(result.reasons.iter().any(|r| r.contains("rolled_back")));
    }

    #[test]
    fn stale_deploy_returns_not_live_verdict() {
        let probes = FakeProbes {
            status_stdout:
                r#"{"state":"live","last_deploy_id":"dep-old","last_deploy_age_secs":99999}"#
                    .to_string(),
            status_exit: 0,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert_eq!(result.verdict, Verdict::NotLive);
        assert!(result.reasons.iter().any(|r| r.contains("최근 배포 없음")));
    }

    #[test]
    fn missing_recent_deploy_returns_not_live_verdict() {
        let probes = FakeProbes {
            status_stdout: r#"{"state":"live","last_deploy_id":null}"#.to_string(),
            status_exit: 0,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert_eq!(result.verdict, Verdict::NotLive);
        assert!(result.reasons.iter().any(|r| r.contains("최근 배포 없음")));
    }

    struct TimeoutProbes;

    impl VerifyProbes for TimeoutProbes {
        fn axhub_status(&self, _app_id: &str) -> ProbeResult {
            ProbeResult {
                stdout: String::new(),
                exit_code: 124,
                timed_out: true,
            }
        }

        fn axhub_logs_tail(&self, _app_id: &str, _lines: u32) -> ProbeResult {
            ProbeResult {
                stdout: String::new(),
                exit_code: 124,
                timed_out: true,
            }
        }
    }

    #[test]
    fn probe_timeouts_return_suspect_with_timeout_reasons() {
        let result = run_verify("paydrop", &TimeoutProbes);
        assert_eq!(result.verdict, Verdict::Suspect);
        assert!(result.reasons.iter().any(|r| r.contains("status timeout")));
        assert!(result.reasons.iter().any(|r| r.contains("logs timeout")));
    }

    #[test]
    fn runtime_errors_demote_live_to_suspect() {
        let probes = FakeProbes {
            status_stdout: r#"{"state":"live","last_deploy_id":"dep-x","last_deploy_age_secs":60}"#
                .to_string(),
            status_exit: 0,
            logs_stdout: "ERROR connection refused\nERROR timeout\nINFO ok\n".to_string(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert_eq!(result.verdict, Verdict::Suspect);
        assert_eq!(result.errors.len(), 2);
        assert!(result.reasons.iter().any(|r| r.contains("ERROR/FATAL")));
    }

    #[test]
    fn axhub_status_failure_yields_suspect() {
        let probes = FakeProbes {
            status_stdout: String::new(),
            status_exit: 1,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert!(matches!(
            result.verdict,
            Verdict::Suspect | Verdict::NotLive
        ));
        assert!(result.reasons.iter().any(|r| r.contains("axhub status")));
    }

    #[test]
    fn invalid_json_in_status_yields_suspect_with_parse_reason() {
        let probes = FakeProbes {
            status_stdout: "not json".to_string(),
            status_exit: 0,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert!(matches!(
            result.verdict,
            Verdict::Suspect | Verdict::NotLive
        ));
        assert!(result.reasons.iter().any(|r| r.contains("JSON parse")));
    }

    #[test]
    fn transport_reason_field_surfaces_in_reasons() {
        // Mimics what RealVerifyProbes synthesises when latest_deploy_id
        // lookup fails with auth.token_invalid (review #8). The reason
        // must reach reasons[] verbatim, NOT be silently collapsed to
        // "state = transport_error".
        // Matches what RealVerifyProbes synthesises post-PR-#149: exit_code 0
        // + state="transport_error" + transport_reason field. exit_code 0
        // routes through the JSON-parse branch where transport_reason is
        // extracted into reasons[].
        let probes = FakeProbes {
            status_stdout: r#"{"state":"transport_error","last_deploy_id":null,"transport_reason":"axhub auth 만료 — axhub auth login 으로 재인증해주세요."}"#
                .to_string(),
            status_exit: 0,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert!(
            result.reasons.iter().any(|r| r.contains("axhub auth 만료")),
            "reasons did not include auth-expired text: {:?}",
            result.reasons
        );
    }

    #[test]
    fn no_recent_deploy_constructor_round_trips_through_verify() {
        // PR #149 / review #18: ProbeResult::no_recent_deploy() is the
        // single source of truth for the synthesized literal. run_verify
        // must still parse it and emit the "최근 배포 없음" reason.
        let probes = FakeProbes {
            status_stdout: ProbeResult::no_recent_deploy().stdout,
            status_exit: 0,
            logs_stdout: String::new(),
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert_eq!(result.verdict, Verdict::NotLive);
        assert!(result.reasons.iter().any(|r| r.contains("최근 배포 없음")));
    }

    #[test]
    fn errors_capped_at_three_lines() {
        let many_errors = (0..10)
            .map(|i| format!("ERROR line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let probes = FakeProbes {
            status_stdout: r#"{"state":"live","last_deploy_id":"d","last_deploy_age_secs":1}"#
                .to_string(),
            status_exit: 0,
            logs_stdout: many_errors,
            logs_exit: 0,
        };
        let result = run_verify("paydrop", &probes);
        assert_eq!(result.errors.len(), 3);
    }
}

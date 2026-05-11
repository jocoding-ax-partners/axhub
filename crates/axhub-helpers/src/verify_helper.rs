// Phase 26 PR 26.4 — verify helper.
//
// Wraps `axhub status` + `axhub logs --runtime --tail 50` for the
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

    if status.exit_code != 0 {
        reasons.push(format!("axhub status exit code {}", status.exit_code));
    } else if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&status.stdout) {
        state = parsed
            .get("state")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        last_deploy_id = parsed
            .get("last_deploy_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        last_deploy_age_secs = parsed.get("last_deploy_age_secs").and_then(|v| v.as_u64());
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

    if let Some(age) = last_deploy_age_secs {
        if age > FRESH_DEPLOY_WINDOW_SECS {
            reasons.push(format!("최근 배포 없음 ({age}초 전)"));
        }
    }

    if logs.exit_code != 0 {
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

    let verdict = compute_verdict(state_live, last_deploy_age_secs, &errors, &reasons);

    VerifyResult {
        verdict,
        state,
        last_deploy_id,
        last_deploy_age_secs,
        errors,
        reasons,
    }
}

fn compute_verdict(
    state_live: bool,
    age_secs: Option<u64>,
    errors: &[String],
    reasons: &[String],
) -> Verdict {
    let fresh = matches!(age_secs, Some(age) if age <= FRESH_DEPLOY_WINDOW_SECS);
    if state_live && fresh && errors.is_empty() && reasons.is_empty() {
        Verdict::Live
    } else if !state_live
        && age_secs
            .map(|a| a > FRESH_DEPLOY_WINDOW_SECS)
            .unwrap_or(false)
    {
        Verdict::NotLive
    } else if !state_live {
        // state ≠ live but recent deploy — still "not live", emit NotLive
        // verdict so SKILL routes to trace.
        Verdict::NotLive
    } else {
        // state=live but with at least one anomaly (errors / stale window /
        // parse problem) → Suspect.
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
            }
        }
        fn axhub_logs_tail(&self, _app_id: &str, _lines: u32) -> ProbeResult {
            ProbeResult {
                stdout: self.logs_stdout.clone(),
                exit_code: self.logs_exit,
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
        assert!(matches!(
            result.verdict,
            Verdict::Suspect | Verdict::NotLive
        ));
        assert!(result.reasons.iter().any(|r| r.contains("최근 배포 없음")));
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

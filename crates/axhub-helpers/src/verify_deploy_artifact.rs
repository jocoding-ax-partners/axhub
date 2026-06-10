//! User-app deploy artifact sanity verifier.
//!
//! Boundary: this module verifies the JSON-ish stdout returned by
//! `axhub deploy create --json` for a user's app. It intentionally does not
//! share code with release artifact verification (`scripts/release-check.ts`),
//! which validates axhub helper release binaries. The verifier is advisory and
//! fail-open: absence of parseable object JSON means "no signal", not failure.
//!
//! Three-value classification (Phase 25 follow-up): the verify hook observes
//! deploy stdout *immediately* after the command returns. Async deploys may
//! legitimately report an in-flight state (queued/building/accepted) at that
//! moment — treating those as failures would break the normal async flow.
//! Conversely, asserting "success" on an absent/unknown state silently hid
//! async 202 responses as passes. So we distinguish three outcomes:
//!   - `Confirmed`   — a terminal success state we recognise; silent pass.
//!   - `Violation`   — a terminal failure or a malformed required field.
//!   - `Unconfirmed` — async in-flight, missing, or unknown signal; advisory.

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// Terminal success — we recognise a live/succeeded state. Silent pass.
    Confirmed,
    /// Terminal failure or a malformed required field (manifest_hash/url/id).
    Violation,
    /// Async in-flight or no usable signal — advisory only, never a hard fail.
    Unconfirmed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyResult {
    pub outcome: VerifyOutcome,
    /// Populated for `Violation` (해요체). Empty otherwise.
    pub violations: Vec<String>,
    /// Populated for `Unconfirmed` (해요체). Empty otherwise.
    pub advisories: Vec<String>,
}

/// Conservative state classification. The backend's async 202 response shape
/// is not documented in this codebase, so only known terminal-success tokens
/// are `TerminalSuccess`; unknown tokens fall to `Absent` (treated as
/// Unconfirmed, never a violation). This guarantees a false "success" is
/// blocked while a normal async deploy is not broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateClass {
    TerminalSuccess,
    TerminalFailure,
    InFlight,
    Absent,
}

fn classify_state(state: Option<&str>) -> StateClass {
    match state {
        None => StateClass::Absent,
        Some(s) => match s.to_lowercase().as_str() {
            "live" | "running" | "deployed" | "active" | "ok" | "succeeded" | "success" => {
                StateClass::TerminalSuccess
            }
            "failed" | "error" | "errored" | "rolled_back" | "cancelled" | "canceled"
            | "crashed" => StateClass::TerminalFailure,
            "pending" | "queued" | "building" | "deploying" | "in_progress" | "accepted"
            | "preparing" | "prepare" => StateClass::InFlight,
            _ => StateClass::Absent,
        },
    }
}

pub fn verify_user_app_artifact(deploy_stdout: &str) -> VerifyResult {
    let Some(response) = parse_deploy_response(deploy_stdout) else {
        // Gap 1: non-JSON / non-object stdout. We have no signal to confirm
        // success — downgrade to advisory instead of asserting a pass.
        return unconfirmed(
            "배포 응답이 JSON 이 아니라 결과를 확인 못 했어요. `axhub deploy logs` 로 확인해요.",
        );
    };

    // manifest_hash format check runs in all branches: a broken hash is a
    // structural anomaly even when the state reports success.
    let mut violations = Vec::new();
    if let Some(manifest_hash) = response.get("manifest_hash") {
        if !manifest_hash.as_str().is_some_and(is_sha256_hex) {
            violations.push("manifest_hash 형식이 sha256 hex (64자 hex) 가 아니에요".to_string());
        }
    }

    // `state` is canonical; fall back to `status` when absent.
    let state_value = response.get("state").or_else(|| response.get("status"));
    let state_str = state_value.map(js_string);

    match classify_state(state_str.as_deref()) {
        StateClass::TerminalSuccess => {
            // Recognised terminal success — keep the existing url/id format
            // checks. Any format violation (here or in manifest_hash) promotes
            // the outcome to Violation even though the state looks healthy.
            check_url(&response, &mut violations);
            check_deploy_id(&response, &mut violations);
            if violations.is_empty() {
                VerifyResult {
                    outcome: VerifyOutcome::Confirmed,
                    violations,
                    advisories: Vec::new(),
                }
            } else {
                violation(violations)
            }
        }
        StateClass::TerminalFailure => {
            // Terminal failure: report the failure plus any format violations
            // (url/id format checks still apply on a failed deploy response).
            check_url(&response, &mut violations);
            check_deploy_id(&response, &mut violations);
            let observed = state_str.as_deref().unwrap_or("unknown");
            violations.push(format!("배포 상태가 \"{observed}\" 예요 (실패)."));
            violation(violations)
        }
        StateClass::InFlight => {
            // Normal async deploy in progress. A broken manifest_hash here is
            // still a structural violation; otherwise advisory only.
            if !violations.is_empty() {
                return violation(violations);
            }
            let observed = state_str.as_deref().unwrap_or("unknown");
            unconfirmed(&format!(
                "배포를 접수했어요 (상태: {observed}). 아직 빌드/준비 중이라 최종 결과는 `axhub deploy logs` 로 확인해요."
            ))
        }
        StateClass::Absent => {
            // No state field, or an unknown token. We can't confirm success.
            if !violations.is_empty() {
                return violation(violations);
            }
            unconfirmed(
                "응답에 배포 상태 필드가 없어 성공을 확정 못 했어요. `axhub deploy logs` 로 확인해요.",
            )
        }
    }
}

fn check_url(response: &Map<String, Value>, violations: &mut Vec<String>) {
    if let Some(url) = response.get("url") {
        let valid = url.as_str().is_some_and(|url| {
            let lower = url.to_lowercase();
            lower.starts_with("http://") || lower.starts_with("https://")
        });
        if !valid {
            violations.push(format!(
                "url=\"{}\" 가 http(s):// 로 시작 안 해요",
                js_string(url)
            ));
        }
    }
}

fn check_deploy_id(response: &Map<String, Value>, violations: &mut Vec<String>) {
    for id_key in ["deployment_id", "deploy_id", "id"] {
        if let Some(value) = response.get(id_key) {
            if value.as_str().is_none_or(|id| id.trim().is_empty()) {
                violations.push(format!("{id_key} 가 비어 있어요"));
            }
            break;
        }
    }
}

fn violation(violations: Vec<String>) -> VerifyResult {
    VerifyResult {
        outcome: VerifyOutcome::Violation,
        violations,
        advisories: Vec::new(),
    }
}

fn unconfirmed(advisory: &str) -> VerifyResult {
    VerifyResult {
        outcome: VerifyOutcome::Unconfirmed,
        violations: Vec::new(),
        advisories: vec![advisory.to_string()],
    }
}

fn parse_deploy_response(stdout: &str) -> Option<Map<String, Value>> {
    let trimmed = stdout.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let parsed: Value = serde_json::from_str(trimmed).ok()?;
    match parsed {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.as_bytes().iter().all(u8::is_ascii_hexdigit)
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome_of(stdout: &str) -> VerifyOutcome {
        verify_user_app_artifact(stdout).outcome
    }

    #[test]
    fn gap2_state_absent_is_unconfirmed_not_silent_pass() {
        // QA regression: previously {deployment_id,url} with no state field
        // produced 0 violations → silent pass. Now it must be Unconfirmed.
        let result = verify_user_app_artifact(
            &serde_json::json!({ "deployment_id": "d1", "url": "https://x" }).to_string(),
        );
        assert_eq!(result.outcome, VerifyOutcome::Unconfirmed);
        assert!(result.violations.is_empty());
        assert!(!result.advisories.is_empty());
        assert!(result
            .advisories
            .iter()
            .any(|a| a.contains("axhub deploy logs")));
    }

    #[test]
    fn async_accepted_states_are_unconfirmed() {
        for state in [
            "accepted",
            "queued",
            "pending",
            "building",
            "preparing",
            "in_progress",
        ] {
            let stdout = serde_json::json!({ "state": state }).to_string();
            assert_eq!(
                outcome_of(&stdout),
                VerifyOutcome::Unconfirmed,
                "in-flight state {state:?} must be Unconfirmed, not a violation"
            );
        }
    }

    #[test]
    fn terminal_failure_states_are_violations() {
        for state in ["failed", "error", "rolled_back", "cancelled", "crashed"] {
            let stdout = serde_json::json!({ "state": state }).to_string();
            let result = verify_user_app_artifact(&stdout);
            assert_eq!(
                result.outcome,
                VerifyOutcome::Violation,
                "terminal failure {state:?} must be a Violation"
            );
            assert!(result.violations.iter().any(|v| v.contains(state)));
        }
    }

    #[test]
    fn terminal_success_states_are_confirmed() {
        for state in [
            "live",
            "running",
            "deployed",
            "active",
            "ok",
            "succeeded",
            "success",
        ] {
            let stdout = serde_json::json!({ "state": state }).to_string();
            assert_eq!(
                outcome_of(&stdout),
                VerifyOutcome::Confirmed,
                "terminal success {state:?} must be Confirmed"
            );
        }
    }

    #[test]
    fn unknown_state_token_is_unconfirmed() {
        // Conservative: an unrecognised token is not asserted as failure.
        let stdout = serde_json::json!({ "state": "frobnicating" }).to_string();
        assert_eq!(outcome_of(&stdout), VerifyOutcome::Unconfirmed);
    }

    #[test]
    fn gap1_non_json_plaintext_downgrades_to_unconfirmed() {
        let result = verify_user_app_artifact("Deploy queued");
        assert_eq!(result.outcome, VerifyOutcome::Unconfirmed);
        assert!(result.violations.is_empty());
        assert!(result
            .advisories
            .iter()
            .any(|a| a.contains("axhub deploy logs")));
    }

    #[test]
    fn broken_manifest_hash_with_success_state_is_violation() {
        // A success state does not excuse a structurally broken manifest_hash.
        let result = verify_user_app_artifact(
            &serde_json::json!({ "state": "live", "manifest_hash": "not-a-sha" }).to_string(),
        );
        assert_eq!(result.outcome, VerifyOutcome::Violation);
        assert!(result.violations.iter().any(|v| v.contains("sha256 hex")));
    }

    #[test]
    fn broken_manifest_hash_with_inflight_state_is_violation() {
        // Structural anomaly overrides the advisory in-flight outcome.
        let result = verify_user_app_artifact(
            &serde_json::json!({ "state": "queued", "manifest_hash": "nope" }).to_string(),
        );
        assert_eq!(result.outcome, VerifyOutcome::Violation);
        assert!(result.violations.iter().any(|v| v.contains("sha256 hex")));
    }

    #[test]
    fn success_with_bad_url_is_violation() {
        let result = verify_user_app_artifact(
            &serde_json::json!({ "state": "live", "url": "ftp://wrong" }).to_string(),
        );
        assert_eq!(result.outcome, VerifyOutcome::Violation);
        assert!(result.violations.iter().any(|v| v.contains("http(s)")));
    }

    #[test]
    fn status_field_is_used_when_state_absent() {
        let result = verify_user_app_artifact(&serde_json::json!({ "status": "live" }).to_string());
        assert_eq!(result.outcome, VerifyOutcome::Confirmed);
    }
}

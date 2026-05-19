//! Phase 4F fix application — plan v6 §4.5.
//!
//! Applies a candidate fix derived from the selected hypothesis, then enters
//! LOOP_VERIFY by re-running Phase 1L. If the loop goes green the loop moves
//! on to Phase 5P; if red the orchestrator regresses to HYPOTHESIZE.

use serde::{Deserialize, Serialize};

use super::signal::Signal;
use super::DiagnoseError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixAction {
    pub id: String,
    pub hypothesis_id: String,
    /// Action class (e.g. "rerun-deploy", "clear-cache", "rebuild"). Plan v6
    /// §4.5 — the actual action runner lives in `skills/recover` and is
    /// wrapped here for orchestration.
    pub class: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixOutcome {
    pub action_id: String,
    pub applied: bool,
    pub verify_signal: Option<Signal>,
}

/// Three-state outcome of LOOP_VERIFY for a fix attempt. Distinguishes "fix
/// proved itself green" from "verify never ran / produced no signal," so the
/// orchestrator does NOT silently treat a missing verify as a red regression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyStatus {
    Green,
    Red,
    /// No verify signal was produced — e.g. the LOOP_VERIFY runner errored out
    /// before emitting a Signal. Caller MUST escalate (retry or HITL), not
    /// implicitly route as a failed fix.
    Unknown,
}

impl FixOutcome {
    /// True only when verify produced a green signal. False for both red and
    /// unknown — use `verify_status()` if you need to distinguish those two.
    pub fn is_green(&self) -> bool {
        matches!(self.verify_status(), VerifyStatus::Green)
    }

    pub fn verify_status(&self) -> VerifyStatus {
        match &self.verify_signal {
            Some(s) if s.is_green() => VerifyStatus::Green,
            Some(_) => VerifyStatus::Red,
            None => VerifyStatus::Unknown,
        }
    }
}

/// Apply a fix action and immediately run LOOP_VERIFY. v0.8.0 skeleton does
/// not actually shell out — wiring to `skills/recover` and the real
/// `loop_builder::build_loop` lands during the Day 3 integration pass.
pub fn apply_fix(action: &FixAction, verify_signal: Signal) -> Result<FixOutcome, DiagnoseError> {
    Ok(FixOutcome {
        action_id: action.id.clone(),
        applied: true,
        verify_signal: Some(verify_signal),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn green_signal_is_green_outcome() {
        let action = FixAction {
            id: "F1".into(),
            hypothesis_id: "H1".into(),
            class: "rerun-deploy".into(),
            description: "재실행 후 검증".into(),
        };
        let sig = Signal::green(Duration::from_millis(500), "cli-replay");
        let outcome = apply_fix(&action, sig).unwrap();
        assert!(outcome.applied);
        assert!(outcome.is_green());
    }

    #[test]
    fn red_signal_is_not_green() {
        let action = FixAction {
            id: "F2".into(),
            hypothesis_id: "H2".into(),
            class: "clear-cache".into(),
            description: "캐시 정리".into(),
        };
        let sig = Signal::red(Duration::from_millis(1500), "cli-replay", None, Some(1));
        let outcome = apply_fix(&action, sig).unwrap();
        assert!(outcome.applied);
        assert!(!outcome.is_green());
        assert_eq!(outcome.verify_status(), VerifyStatus::Red);
    }

    #[test]
    fn missing_signal_is_unknown_not_red() {
        // A future verify-runner failure (network error, helper crash) may
        // produce an outcome with verify_signal=None. That MUST NOT silently
        // pass for "red" — the orchestrator needs the third state to know
        // it should escalate rather than try the next hypothesis.
        let outcome = FixOutcome {
            action_id: "F-missing".into(),
            applied: true,
            verify_signal: None,
        };
        assert_eq!(outcome.verify_status(), VerifyStatus::Unknown);
        assert!(!outcome.is_green());
    }

    #[test]
    fn outcome_serde() {
        let action = FixAction {
            id: "F3".into(),
            hypothesis_id: "H3".into(),
            class: "noop".into(),
            description: "x".into(),
        };
        let sig = Signal::green(Duration::from_millis(1), "test");
        let outcome = apply_fix(&action, sig).unwrap();
        let s = serde_json::to_string(&outcome).unwrap();
        let back: FixOutcome = serde_json::from_str(&s).unwrap();
        assert_eq!(outcome, back);
    }
}

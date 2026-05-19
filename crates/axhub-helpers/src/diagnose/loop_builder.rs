//! Phase 1L loop builder — plan v6 §4.1.
//!
//! Tries strategies in cost-asc order until one yields a deterministic
//! `Signal`. v0.8.0 ships strategies 1 (existing-test), 2 (cli-replay),
//! 3 (trace-replay), and 7 (HITL fallback). Strategies 4-6 (snapshot-diff,
//! bisect, differential) ship later (v0.8.1+ / v0.9).
//!
//! Tool ↔ enum variant mapping (v0.8.0):
//! - `LoopStrategy::AxhubDeploy` — axhub-helpers deploy event_log replay
//! - `LoopStrategy::Test` — `cargo test` / `bun test` / vitest / jest re-run
//!
//! npm install lands in v0.8.1 once the cold-cache prototype confirms a
//! reasonable signal-emergence budget.

use serde::{Deserialize, Serialize};

use super::signal::Signal;
use super::DiagnoseError;

/// Which tool the loop is targeting. v0.8.0 ships two variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum LoopStrategy {
    AxhubDeploy,
    Test,
}

impl LoopStrategy {
    /// Plan v6 §4.1 — signal-emergence target per tool.
    pub fn signal_target_ms(self) -> u64 {
        match self {
            LoopStrategy::AxhubDeploy => 2_000,
            LoopStrategy::Test => 2_000,
        }
    }

    /// Plan v6 §4.1 — loop wall budget per tool.
    pub fn wall_budget_ms(self) -> u64 {
        match self {
            LoopStrategy::AxhubDeploy => 30_000,
            LoopStrategy::Test => 60_000,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LoopStrategy::AxhubDeploy => "axhub-deploy",
            LoopStrategy::Test => "test",
        }
    }
}

/// Result of one `build_loop` attempt — either a deterministic signal or a
/// reason the strategy was inapplicable (so the caller can fall through to
/// HITL).
#[derive(Debug, Clone)]
pub enum BuildOutcome {
    /// Strategy produced a signal.
    Signal(Signal),
    /// Strategy did not apply (precondition missing, e.g. no previous test).
    /// Caller should try the next strategy or HITL.
    NotApplicable(String),
}

/// Build a loop for the requested tool. v0.8.0 implementation is intentionally
/// minimal — real strategy runners land alongside the Phase 4F fix integration.
/// At skeleton stage this returns `NotApplicable` so the orchestrator falls
/// through to HITL, which is the safe default for unknown environments.
pub fn build_loop(strategy: LoopStrategy) -> Result<BuildOutcome, DiagnoseError> {
    Ok(BuildOutcome::NotApplicable(format!(
        "strategy {} not yet implemented; orchestrator must fall through to HITL",
        strategy.as_str()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_round_trip_serde() {
        for s in [LoopStrategy::AxhubDeploy, LoopStrategy::Test] {
            let j = serde_json::to_string(&s).unwrap();
            let back: LoopStrategy = serde_json::from_str(&j).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn budgets_match_plan_v6() {
        // Plan v6 §4.1 budget table — locked-in values.
        assert_eq!(LoopStrategy::AxhubDeploy.signal_target_ms(), 2_000);
        assert_eq!(LoopStrategy::Test.signal_target_ms(), 2_000);
        assert_eq!(LoopStrategy::AxhubDeploy.wall_budget_ms(), 30_000);
        assert_eq!(LoopStrategy::Test.wall_budget_ms(), 60_000);
    }

    #[test]
    fn skeleton_returns_not_applicable() {
        let outcome = build_loop(LoopStrategy::AxhubDeploy).unwrap();
        match outcome {
            BuildOutcome::NotApplicable(reason) => {
                assert!(reason.contains("HITL"), "must hint HITL fallback: {reason}");
            }
            _ => panic!("v0.8.0 skeleton must return NotApplicable"),
        }
    }
}

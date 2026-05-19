//! Phase 4.7 — diagnose pre-flight scan. Plan v6 §4.7.
//!
//! Distinct from `crate::preflight` (deploy preflight). Runs 5 cheap probes
//! in parallel under a 200 ms wall budget and reports findings as a
//! systemMessage candidate.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::DiagnoseError;

pub const WALL_BUDGET_MS: u64 = 200;
pub const PER_CHECK_TIMEOUT_MS: u64 = 50;
pub const WALL_BUDGET_ENV: &str = "AXHUB_DIAGNOSE_PREFLIGHT_WALL_MS";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckOutcome {
    Pass,
    Warn(String),
    Skipped(String),
    Failed(String),
}

impl CheckOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckOutcome::Pass)
    }
    pub fn is_warn(&self) -> bool {
        matches!(self, CheckOutcome::Warn(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckReport {
    pub name: String,
    pub outcome: CheckOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightSummary {
    pub reports: Vec<CheckReport>,
    pub wall_exceeded: bool,
    pub wall_ms: u64,
}

impl PreflightSummary {
    pub fn warnings(&self) -> Vec<&CheckReport> {
        self.reports
            .iter()
            .filter(|r| r.outcome.is_warn())
            .collect()
    }
}

pub fn effective_wall_budget() -> Duration {
    let ms = std::env::var(WALL_BUDGET_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(WALL_BUDGET_MS);
    Duration::from_millis(ms)
}

use std::future::Future;
use std::pin::Pin;

/// Boxed future type so all 5 checks have the same concrete type when joined.
pub type BoxedCheck = Pin<Box<dyn Future<Output = CheckOutcome> + Send>>;

/// Run all builtin checks in parallel. The closures are owned by the caller
/// so tests can inject fast deterministic stubs without touching disk or
/// network.
pub async fn run_checks(
    disk_check: BoxedCheck,
    node_check: BoxedCheck,
    npm_cache_check: BoxedCheck,
    git_check: BoxedCheck,
    helper_version_check: BoxedCheck,
) -> Result<PreflightSummary, DiagnoseError> {
    let budget = effective_wall_budget();
    let started = std::time::Instant::now();

    let (disk, node, cache, git, helper) = tokio::join!(
        tokio::time::timeout(budget, disk_check),
        tokio::time::timeout(budget, node_check),
        tokio::time::timeout(budget, npm_cache_check),
        tokio::time::timeout(budget, git_check),
        tokio::time::timeout(budget, helper_version_check),
    );

    let wall_ms = started.elapsed().as_millis() as u64;
    let wall_exceeded = wall_ms > budget.as_millis() as u64;

    fn flatten(name: &str, r: Result<CheckOutcome, tokio::time::error::Elapsed>) -> CheckReport {
        let outcome = match r {
            Ok(o) => o,
            Err(_) => CheckOutcome::Skipped("wall budget exceeded".into()),
        };
        CheckReport {
            name: name.into(),
            outcome,
        }
    }

    Ok(PreflightSummary {
        reports: vec![
            flatten("disk_free", disk),
            flatten("node_version", node),
            flatten("npm_cache_health", cache),
            flatten("git_clean_tree", git),
            flatten("axhub_helper_version", helper),
        ],
        wall_exceeded,
        wall_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fast_pass() -> CheckOutcome {
        CheckOutcome::Pass
    }

    async fn fast_warn() -> CheckOutcome {
        CheckOutcome::Warn("disk_free=412MB".into())
    }

    async fn slow_pass() -> CheckOutcome {
        tokio::time::sleep(Duration::from_millis(400)).await;
        CheckOutcome::Pass
    }

    #[tokio::test]
    async fn all_pass_yields_no_warnings() {
        let summary = run_checks(
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
        )
        .await
        .unwrap();
        assert_eq!(summary.reports.len(), 5);
        assert!(summary.warnings().is_empty());
        assert!(!summary.wall_exceeded);
    }

    #[tokio::test]
    async fn warn_check_appears_in_warnings() {
        let summary = run_checks(
            Box::pin(fast_warn()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
        )
        .await
        .unwrap();
        assert_eq!(summary.warnings().len(), 1);
        assert_eq!(summary.warnings()[0].name, "disk_free");
    }

    #[tokio::test]
    async fn slow_check_skipped_under_budget() {
        let _g = crate::PROCESS_ENV_LOCK.lock().unwrap();
        std::env::set_var(WALL_BUDGET_ENV, "100");
        let summary = run_checks(
            Box::pin(slow_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
            Box::pin(fast_pass()),
        )
        .await
        .unwrap();
        std::env::remove_var(WALL_BUDGET_ENV);
        // disk_free check should be skipped because it slept 400ms over 100ms budget.
        let disk = summary
            .reports
            .iter()
            .find(|r| r.name == "disk_free")
            .unwrap();
        assert!(matches!(disk.outcome, CheckOutcome::Skipped(_)));
    }

    #[test]
    fn effective_wall_budget_default() {
        let _g = crate::PROCESS_ENV_LOCK.lock().unwrap();
        std::env::remove_var(WALL_BUDGET_ENV);
        assert_eq!(effective_wall_budget().as_millis() as u64, WALL_BUDGET_MS);
    }

    #[test]
    fn outcome_helpers() {
        assert!(CheckOutcome::Pass.is_pass());
        assert!(!CheckOutcome::Pass.is_warn());
        let w = CheckOutcome::Warn("x".into());
        assert!(w.is_warn());
        assert!(!w.is_pass());
    }
}

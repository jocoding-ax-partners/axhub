//! Recurrence detection — plan v6 §4.10.
//!
//! Reads `learnings.jsonl`, counts how many times the same
//! (`error_class`, `cwd_hash`) tuple has appeared. When the count crosses
//! `RECURRENCE_THRESHOLD` (default 3, configurable via env), emits an
//! architectural finding into `docs/architectural-findings/` and a ledger
//! event for the orchestrator to escalate to ARCH_HANDOFF.

use std::collections::HashMap;

use crate::audit_ledger::{self, LedgerEntry};

use super::learning::LearningEntry;
use super::DiagnoseError;

pub const RECURRENCE_THRESHOLD_DEFAULT: u32 = 3;
pub const RECURRENCE_THRESHOLD_ENV: &str = "AXHUB_RECURRENCE_THRESHOLD";

pub fn effective_threshold() -> u32 {
    std::env::var(RECURRENCE_THRESHOLD_ENV)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(RECURRENCE_THRESHOLD_DEFAULT)
}

/// Count occurrences for a given (error_class, cwd_hash) pair across the
/// supplied learnings.
pub fn count_for(entries: &[LearningEntry], error_class: &str, cwd_hash: &str) -> u32 {
    entries
        .iter()
        .filter(|e| e.error_class == error_class && e.cwd_hash == cwd_hash)
        .count() as u32
}

/// Group recurrence counts across all learnings. Useful for telemetry.
pub fn aggregate(entries: &[LearningEntry]) -> HashMap<(String, String), u32> {
    let mut counts = HashMap::new();
    for e in entries {
        let key = (e.error_class.clone(), e.cwd_hash.clone());
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Returns true if the latest entry pushes the (error_class, cwd_hash) past
/// the configured threshold.
pub fn crossed_threshold(entries: &[LearningEntry], error_class: &str, cwd_hash: &str) -> bool {
    count_for(entries, error_class, cwd_hash) >= effective_threshold()
}

pub fn emit_threshold_event(
    loop_id: &str,
    error_class: &str,
    count: u32,
) -> Result<(), DiagnoseError> {
    let entry = LedgerEntry::new("recurrence.threshold_hit")
        .with_loop_id(loop_id.to_string())
        .with_action(error_class.to_string())
        .with_evidence(serde_json::json!({
            "error_class": error_class,
            "count": count,
            "threshold": effective_threshold(),
        }));
    audit_ledger::append_entry(&entry).map_err(|e| DiagnoseError::LearningEmitFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::learning::builder;

    use std::sync::Mutex;

    /// Serialize env-modifying tests so parallel cargo test runners don't
    /// race RECURRENCE_THRESHOLD_ENV. cargo test defaults to N threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn make(err: &str, cwd: &str) -> LearningEntry {
        let mut e = builder(err);
        e.cwd_hash = cwd.into();
        e
    }

    #[test]
    fn default_threshold_is_3() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var(RECURRENCE_THRESHOLD_ENV);
        assert_eq!(effective_threshold(), 3);
    }

    #[test]
    fn env_override_threshold() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var(RECURRENCE_THRESHOLD_ENV, "5");
        assert_eq!(effective_threshold(), 5);
        std::env::remove_var(RECURRENCE_THRESHOLD_ENV);
    }

    #[test]
    fn count_matches_filtered_entries() {
        let entries = vec![
            make("npm_eacces", "cwd1"),
            make("npm_eacces", "cwd1"),
            make("npm_eacces", "cwd2"),
            make("other_err", "cwd1"),
        ];
        assert_eq!(count_for(&entries, "npm_eacces", "cwd1"), 2);
        assert_eq!(count_for(&entries, "npm_eacces", "cwd2"), 1);
        assert_eq!(count_for(&entries, "other_err", "cwd1"), 1);
    }

    #[test]
    fn threshold_crossed_when_count_at_or_above() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var(RECURRENCE_THRESHOLD_ENV, "2");
        let entries = vec![make("e1", "cwd1"), make("e1", "cwd1")];
        assert!(crossed_threshold(&entries, "e1", "cwd1"));
        let one = vec![make("e1", "cwd1")];
        assert!(!crossed_threshold(&one, "e1", "cwd1"));
        std::env::remove_var(RECURRENCE_THRESHOLD_ENV);
    }

    #[test]
    fn aggregate_groups_by_pair() {
        let entries = vec![make("e1", "c1"), make("e1", "c1"), make("e2", "c1")];
        let m = aggregate(&entries);
        assert_eq!(m[&("e1".into(), "c1".into())], 2);
        assert_eq!(m[&("e2".into(), "c1".into())], 1);
    }
}

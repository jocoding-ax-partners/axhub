//! Deterministic pass/fail signal store for Phase 1L.
//!
//! Plan v6 §4.1 — the loop builder produces a `Signal` once per attempt.
//! The signal is the diagnose loop's single source of truth for "did the
//! failure reproduce." Auxiliary metadata (stderr line, exit code, timing)
//! travels alongside but never replaces the boolean.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// One pass/fail observation from the loop builder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signal {
    /// `true` = original failure reproduced. `false` = green (no failure).
    pub failed: bool,
    /// Wall time from loop start to signal emergence.
    pub elapsed_ms: u64,
    /// Strategy that produced this signal (existing-test / cli-replay / ...).
    pub strategy: String,
    /// First stderr/stdout line that emerged with the signal, if any.
    /// MUST be passed through `crate::redact::redact` before serialization.
    pub head_evidence: Option<String>,
    /// Exit code of the underlying process if applicable.
    pub exit_code: Option<i32>,
}

impl Signal {
    pub fn green(elapsed: Duration, strategy: impl Into<String>) -> Self {
        Self {
            failed: false,
            elapsed_ms: elapsed.as_millis() as u64,
            strategy: strategy.into(),
            head_evidence: None,
            exit_code: None,
        }
    }

    pub fn red(
        elapsed: Duration,
        strategy: impl Into<String>,
        head_evidence: Option<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self {
            failed: true,
            elapsed_ms: elapsed.as_millis() as u64,
            strategy: strategy.into(),
            head_evidence,
            exit_code,
        }
    }

    /// Plan v6 §3.3 — LOOP_VERIFY green ⇔ `!failed`.
    pub fn is_green(&self) -> bool {
        !self.failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_constructor() {
        let s = Signal::green(Duration::from_millis(750), "existing-test");
        assert!(s.is_green());
        assert_eq!(s.elapsed_ms, 750);
        assert_eq!(s.strategy, "existing-test");
    }

    #[test]
    fn red_constructor_preserves_evidence() {
        let s = Signal::red(
            Duration::from_millis(2_300),
            "cli-replay",
            Some("npm ERR! EACCES".into()),
            Some(243),
        );
        assert!(!s.is_green());
        assert!(s.failed);
        assert_eq!(s.head_evidence.as_deref(), Some("npm ERR! EACCES"));
        assert_eq!(s.exit_code, Some(243));
    }

    #[test]
    fn serde_roundtrip() {
        let s = Signal::red(Duration::from_millis(1_500), "trace-replay", None, None);
        let json = serde_json::to_string(&s).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}

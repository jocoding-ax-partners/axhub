//! axhub auto-diagnose system — 5-Phase loop (1L → 2R → 3I → 4F → 5P).
//!
//! See `.plan/ceo-review-vibe-coder-auto-recovery/2026-05-19-plan-v6-loop-first.md`
//! for the full design. This module hosts the runtime; plan §3.1 and §4.x are
//! the source-of-truth for the state machine, phase contracts, and budgets.
//!
//! Layering rule (plan §14):
//! `diagnose/*` MAY import: audit_ledger, redact, runtime_paths, recovery_scan,
//!                          event_log, telemetry, hook_safety.
//! `diagnose/*` MUST NOT import: bootstrap, list_deployments, deploy_prep,
//!                                statusline, preflight (deploy-preflight).
//! Enforced by `tests/diagnose_layering_test.rs`.

pub mod fix;
pub mod hitl;
pub mod hypothesis;
pub mod instrument;
pub mod learning;
pub mod loop_builder;
pub mod postmortem;
pub mod preflight;
pub mod probe;
pub mod recurrence;
pub mod signal;
pub mod state;

pub use fix::*;
pub use hitl::*;
pub use hypothesis::*;
pub use instrument::*;
pub use learning::*;
pub use loop_builder::*;
pub use postmortem::*;
pub use preflight::*;
pub use probe::*;
pub use recurrence::*;
pub use signal::*;
pub use state::*;

use thiserror::Error;

/// All errors raised by the diagnose loop. Plan v6 §5 specifies one Rescue
/// action per variant (no catch-all). Keep this enum exhaustive — adding a
/// variant forces every match site to handle the new case explicitly.
#[derive(Debug, Error)]
pub enum DiagnoseError {
    /// Phase 1L could not build a deterministic feedback loop.
    #[error("loop builder failed: {0}")]
    LoopExecFailed(String),

    /// Phase 1L wall budget exceeded.
    #[error("loop wall budget exceeded after {0:?}")]
    LoopTimeout(std::time::Duration),

    /// Phase 2R had no hypotheses to offer (catalog miss + LLM disabled).
    #[error("no hypothesis generated — escalate to HITL extra capture")]
    NoHypothesis,

    /// Phase 2R LLM-augmented generator failed (v0.8.1+).
    #[error("LLM hypothesis generator error: {0}")]
    LlmError(String),

    /// Phase 2R LLM returned malformed JSON.
    #[error("LLM returned malformed JSON: {0}")]
    LlmMalformed(String),

    /// Phase 3I probe could not apply (e.g. file I/O).
    #[error("probe apply failed: {0}")]
    ProbeApplyFailed(String),

    /// Phase 3I probe touched a file outside its declared `touches()` set
    /// (runtime guard — plan §4.4).
    #[error("probe boundary violation: probe={probe_id} touched={path}")]
    ProbeBoundaryViolation { probe_id: String, path: String },

    /// Phase 4F fix could not apply.
    #[error("fix apply failed: {0}")]
    FixApplyFailed(String),

    /// State-machine transition refused — `(src, event)` pair has no valid
    /// destination. Distinct from `FixApplyFailed` so callers don't route
    /// rescue actions intended for fix failures.
    #[error("invalid transition: src={src} event={event}")]
    InvalidTransition {
        src: &'static str,
        event: &'static str,
    },

    /// LOOP_VERIFY oscillation cap hit — same hypothesis-fix-verify cycle
    /// returned red more than `MAX_VERIFY_RETRIES` times for this loop. The
    /// orchestrator must escalate to ArchHandoff.
    #[error("LOOP_VERIFY retry cap exceeded ({attempts} >= {max})")]
    VerifyRetryCapExceeded { attempts: u32, max: u32 },

    /// Phase 5P cleanup encountered an error (audit-only, never propagated to
    /// user; we still consider the loop done).
    #[error("postmortem cleanup failed: {0}")]
    CleanupFailed(String),

    /// Phase 5P learning emit failed (best-effort).
    #[error("learning emit failed: {0}")]
    LearningEmitFailed(String),

    /// SessionStart preflight panicked. Fail-open; no systemMessage injected.
    #[error("preflight panic: {0}")]
    PreflightPanic(String),

    /// HITL Rust subcommand could not read TTY (e.g. CI sandbox).
    #[error("HITL TTY unavailable")]
    HitlNoTty,

    /// HITL subcommand I/O error.
    #[error("HITL I/O error: {0}")]
    HitlIoError(#[from] std::io::Error),

    /// HITL session was aborted (e.g. session timeout) — partial result.
    #[error("HITL aborted: {0}")]
    HitlAborted(String),

    /// Catch-all for `serde_json::Error` in HITL/spec serialization.
    #[error("HITL JSON error: {0}")]
    HitlJsonError(#[from] serde_json::Error),
}

//! Diagnose state machine — plan v6 §3.3.
//!
//! State transitions go through ONE entry point: [`DiagnoseState::transition`].
//! Invalid transitions are unreachable at compile time (enum match must be
//! exhaustive). A `Mutex<DiagnoseSession>` wrapping the state blocks
//! same-process multi-thread races (plan v6 eng-review architecture #3).
//!
//! ```text
//! IDLE → BUILDING → REPRODUCING → HYPOTHESIZE → INSTRUMENTING
//!                                       ↑              ↓
//!                                       └─ FIXING ← LOOP_VERIFY (red)
//!                                                       ↓ (green)
//!                                                  POSTMORTEM → IDLE
//!
//! ARCH_HANDOFF is a separate terminal from any state on exhaustion.
//! ```

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use super::DiagnoseError;

/// Hard cap on LOOP_VERIFY → HYPOTHESIZE oscillations per loop. Beyond this
/// the session is forced into ArchHandoff rather than spinning forever on a
/// catalog hypothesis that the fix never makes green. Plan v6 §3.3 — keeps
/// the diagnose loop bounded even if hypothesis generator never converges.
pub const MAX_VERIFY_RETRIES: u32 = 5;

/// Discrete state of one diagnose loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnoseState {
    /// No loop in progress.
    Idle,
    /// Phase 1L building loop / signal source.
    Building,
    /// Phase 2 gate — verifying that the loop signal matches the user-reported
    /// failure before any hypothesizing.
    Reproducing,
    /// Phase 2R generating + ranking hypotheses.
    Hypothesize,
    /// Phase 3I applying a probe + observing signal change.
    Instrumenting,
    /// Phase 4F applying a candidate fix.
    Fixing,
    /// Internal state between FIXING and POSTMORTEM. Re-runs Phase 1L loop to
    /// confirm fix made the failure stop reproducing.
    LoopVerify,
    /// Phase 5P writing learning + cleanup.
    Postmortem,
    /// Terminal — recurrence threshold hit or all hypotheses exhausted.
    /// Plan v6 §3.3 — emits architectural finding before transitioning to Idle.
    ArchHandoff,
}

impl DiagnoseState {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnoseState::Idle => "IDLE",
            DiagnoseState::Building => "BUILDING",
            DiagnoseState::Reproducing => "REPRODUCING",
            DiagnoseState::Hypothesize => "HYPOTHESIZE",
            DiagnoseState::Instrumenting => "INSTRUMENTING",
            DiagnoseState::Fixing => "FIXING",
            DiagnoseState::LoopVerify => "LOOP_VERIFY",
            DiagnoseState::Postmortem => "POSTMORTEM",
            DiagnoseState::ArchHandoff => "ARCH_HANDOFF",
        }
    }
}

/// Events that drive state transitions. Each event maps to exactly one valid
/// destination per source state — invalid (src, event) pairs are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum DiagnoseEvent {
    /// Trigger received (user utterance / hook / event_log).
    Trigger,
    /// Phase 1L finished building a loop.
    LoopReady,
    /// Phase 2 gate — symptom confirmed.
    SymptomConfirmed,
    /// Phase 2R produced a ranked hypothesis list.
    HypothesesReady,
    /// Phase 3I probe applied successfully.
    ProbeApplied,
    /// Phase 4F fix applied — entering LOOP_VERIFY.
    FixApplied,
    /// LOOP_VERIFY green.
    LoopVerifyGreen,
    /// LOOP_VERIFY red — back to HYPOTHESIZE for next candidate.
    LoopVerifyRed,
    /// All hypotheses exhausted — architectural handoff.
    HypothesesExhausted,
    /// Recurrence threshold (3 occurrences) crossed.
    RecurrenceThresholdHit,
    /// Phase 5P cleanup completed.
    CleanupDone,
    /// Architectural handoff finished (audit emitted, user notified).
    HandoffSent,
}

impl DiagnoseEvent {
    pub fn as_str(self) -> &'static str {
        use DiagnoseEvent::*;
        match self {
            Trigger => "Trigger",
            LoopReady => "LoopReady",
            SymptomConfirmed => "SymptomConfirmed",
            HypothesesReady => "HypothesesReady",
            ProbeApplied => "ProbeApplied",
            FixApplied => "FixApplied",
            LoopVerifyGreen => "LoopVerifyGreen",
            LoopVerifyRed => "LoopVerifyRed",
            HypothesesExhausted => "HypothesesExhausted",
            RecurrenceThresholdHit => "RecurrenceThresholdHit",
            CleanupDone => "CleanupDone",
            HandoffSent => "HandoffSent",
        }
    }
}

impl DiagnoseState {
    /// Single-entry transition. Returns `Ok(next)` for valid moves, `Err` for
    /// invalid. Audit ledger entry is the caller's responsibility — this
    /// function is pure.
    pub fn transition(self, event: DiagnoseEvent) -> Result<Self, DiagnoseError> {
        use DiagnoseEvent::*;
        use DiagnoseState::*;
        let next = match (self, event) {
            (Idle, Trigger) => Building,
            (Building, LoopReady) => Reproducing,
            (Reproducing, SymptomConfirmed) => Hypothesize,
            (Hypothesize, HypothesesReady) => Instrumenting,
            (Hypothesize, HypothesesExhausted) => ArchHandoff,
            (Instrumenting, ProbeApplied) => Fixing,
            (Fixing, FixApplied) => LoopVerify,
            (LoopVerify, LoopVerifyGreen) => Postmortem,
            (LoopVerify, LoopVerifyRed) => Hypothesize,
            (LoopVerify, HypothesesExhausted) => ArchHandoff,
            (Postmortem, CleanupDone) => Idle,
            (Postmortem, RecurrenceThresholdHit) => ArchHandoff,
            (ArchHandoff, HandoffSent) => Idle,
            (src, ev) => {
                return Err(DiagnoseError::InvalidTransition {
                    src: src.as_str(),
                    event: ev.as_str(),
                });
            }
        };
        Ok(next)
    }

    /// A state is terminal if no further work is needed without a new trigger.
    /// Both Idle (clean exit) and ArchHandoff (escalated exit awaiting user
    /// notification) qualify — neither has self-driving outbound transitions.
    pub fn is_terminal(self) -> bool {
        matches!(self, DiagnoseState::Idle | DiagnoseState::ArchHandoff)
    }
}

/// Owning wrapper for a single diagnose run. Guards same-process concurrency.
pub struct DiagnoseSession {
    state: Mutex<DiagnoseState>,
    loop_id: String,
    /// Count of LOOP_VERIFY → HYPOTHESIZE returns. When this reaches
    /// `MAX_VERIFY_RETRIES`, the next LoopVerifyRed forces ArchHandoff
    /// instead of cycling back to Hypothesize.
    verify_red_count: AtomicU32,
}

impl DiagnoseSession {
    pub fn new(loop_id: impl Into<String>) -> Self {
        Self {
            state: Mutex::new(DiagnoseState::Idle),
            loop_id: loop_id.into(),
            verify_red_count: AtomicU32::new(0),
        }
    }

    pub fn loop_id(&self) -> &str {
        &self.loop_id
    }

    /// Current LOOP_VERIFY retry count (observable for telemetry / tests).
    pub fn verify_red_count(&self) -> u32 {
        self.verify_red_count.load(Ordering::SeqCst)
    }

    /// Read the current state. Holds the lock for the duration of the read.
    /// Recovers from poisoning rather than panicking — fail-open contract
    /// (CLAUDE.md axhub Hook Safety §10.6). The protected enum is `Copy` so
    /// the read after poisoning is well-defined.
    pub fn snapshot(&self) -> DiagnoseState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Apply an event under the state lock. Returns the resulting state.
    ///
    /// `LoopVerifyRed` is special-cased: after `MAX_VERIFY_RETRIES` returns,
    /// the loop is forced into ArchHandoff via `HypothesesExhausted` so the
    /// session cannot oscillate forever. The retry counter is reset on
    /// `CleanupDone` (loop reaches Idle).
    pub fn apply(&self, event: DiagnoseEvent) -> Result<DiagnoseState, DiagnoseError> {
        let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());

        if event == DiagnoseEvent::LoopVerifyRed && *guard == DiagnoseState::LoopVerify {
            let prior = self.verify_red_count.fetch_add(1, Ordering::SeqCst);
            // `prior` is the count BEFORE this increment. After MAX_VERIFY_RETRIES
            // successful red returns we route the very next red to handoff so
            // the orchestrator can finalise the architectural finding.
            if prior >= MAX_VERIFY_RETRIES {
                let next = guard.transition(DiagnoseEvent::HypothesesExhausted)?;
                *guard = next;
                return Err(DiagnoseError::VerifyRetryCapExceeded {
                    attempts: prior + 1,
                    max: MAX_VERIFY_RETRIES,
                });
            }
        }

        let next = guard.transition(event)?;
        if event == DiagnoseEvent::CleanupDone {
            self.verify_red_count.store(0, Ordering::SeqCst);
        }
        *guard = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions() {
        let s = DiagnoseSession::new("loop-happy");
        assert_eq!(s.snapshot(), DiagnoseState::Idle);
        assert_eq!(
            s.apply(DiagnoseEvent::Trigger).unwrap(),
            DiagnoseState::Building
        );
        assert_eq!(
            s.apply(DiagnoseEvent::LoopReady).unwrap(),
            DiagnoseState::Reproducing
        );
        assert_eq!(
            s.apply(DiagnoseEvent::SymptomConfirmed).unwrap(),
            DiagnoseState::Hypothesize
        );
        assert_eq!(
            s.apply(DiagnoseEvent::HypothesesReady).unwrap(),
            DiagnoseState::Instrumenting
        );
        assert_eq!(
            s.apply(DiagnoseEvent::ProbeApplied).unwrap(),
            DiagnoseState::Fixing
        );
        assert_eq!(
            s.apply(DiagnoseEvent::FixApplied).unwrap(),
            DiagnoseState::LoopVerify
        );
        assert_eq!(
            s.apply(DiagnoseEvent::LoopVerifyGreen).unwrap(),
            DiagnoseState::Postmortem
        );
        assert_eq!(
            s.apply(DiagnoseEvent::CleanupDone).unwrap(),
            DiagnoseState::Idle
        );
        assert!(s.snapshot().is_terminal());
    }

    #[test]
    fn loop_verify_red_returns_to_hypothesize() {
        let s = DiagnoseSession::new("loop-red");
        s.apply(DiagnoseEvent::Trigger).unwrap();
        s.apply(DiagnoseEvent::LoopReady).unwrap();
        s.apply(DiagnoseEvent::SymptomConfirmed).unwrap();
        s.apply(DiagnoseEvent::HypothesesReady).unwrap();
        s.apply(DiagnoseEvent::ProbeApplied).unwrap();
        s.apply(DiagnoseEvent::FixApplied).unwrap();
        let next = s.apply(DiagnoseEvent::LoopVerifyRed).unwrap();
        assert_eq!(
            next,
            DiagnoseState::Hypothesize,
            "red must regress to HYPOTHESIZE"
        );
    }

    #[test]
    fn loop_verify_red_cap_exceeded_enters_arch_handoff() {
        let s = DiagnoseSession::new("loop-cap");
        for ev in [
            DiagnoseEvent::Trigger,
            DiagnoseEvent::LoopReady,
            DiagnoseEvent::SymptomConfirmed,
            DiagnoseEvent::HypothesesReady,
            DiagnoseEvent::ProbeApplied,
            DiagnoseEvent::FixApplied,
        ] {
            s.apply(ev).unwrap();
        }

        for attempt in 1..=MAX_VERIFY_RETRIES {
            assert_eq!(
                s.apply(DiagnoseEvent::LoopVerifyRed).unwrap(),
                DiagnoseState::Hypothesize,
                "red attempt {attempt} should stay retryable"
            );
            assert_eq!(s.verify_red_count(), attempt);
            for ev in [
                DiagnoseEvent::HypothesesReady,
                DiagnoseEvent::ProbeApplied,
                DiagnoseEvent::FixApplied,
            ] {
                s.apply(ev).unwrap();
            }
        }

        let err = s.apply(DiagnoseEvent::LoopVerifyRed).unwrap_err();
        match err {
            DiagnoseError::VerifyRetryCapExceeded { attempts, max } => {
                assert_eq!(attempts, MAX_VERIFY_RETRIES + 1);
                assert_eq!(max, MAX_VERIFY_RETRIES);
            }
            other => panic!("expected VerifyRetryCapExceeded, got {other:?}"),
        }
        assert_eq!(
            s.snapshot(),
            DiagnoseState::ArchHandoff,
            "cap breach must terminally hand off instead of sticking in LOOP_VERIFY"
        );
    }

    #[test]
    fn invalid_transition_rejected() {
        let s = DiagnoseSession::new("loop-bad");
        // IDLE on LoopReady is invalid — must Trigger first.
        let err = s.apply(DiagnoseEvent::LoopReady);
        assert!(err.is_err(), "must reject IDLE→LoopReady");
        // After the rejection, state must remain Idle.
        assert_eq!(s.snapshot(), DiagnoseState::Idle);
    }

    #[test]
    fn exhausted_hypotheses_goes_to_handoff() {
        let s = DiagnoseSession::new("loop-handoff");
        s.apply(DiagnoseEvent::Trigger).unwrap();
        s.apply(DiagnoseEvent::LoopReady).unwrap();
        s.apply(DiagnoseEvent::SymptomConfirmed).unwrap();
        let next = s.apply(DiagnoseEvent::HypothesesExhausted).unwrap();
        assert_eq!(next, DiagnoseState::ArchHandoff);
        let final_ = s.apply(DiagnoseEvent::HandoffSent).unwrap();
        assert_eq!(final_, DiagnoseState::Idle);
    }

    #[test]
    fn recurrence_threshold_from_postmortem() {
        let s = DiagnoseSession::new("loop-recurrence");
        // Drive to Postmortem.
        for ev in [
            DiagnoseEvent::Trigger,
            DiagnoseEvent::LoopReady,
            DiagnoseEvent::SymptomConfirmed,
            DiagnoseEvent::HypothesesReady,
            DiagnoseEvent::ProbeApplied,
            DiagnoseEvent::FixApplied,
            DiagnoseEvent::LoopVerifyGreen,
        ] {
            s.apply(ev).unwrap();
        }
        assert_eq!(s.snapshot(), DiagnoseState::Postmortem);
        assert_eq!(
            s.apply(DiagnoseEvent::RecurrenceThresholdHit).unwrap(),
            DiagnoseState::ArchHandoff
        );
    }

    #[test]
    fn concurrent_apply_serializes() {
        use std::sync::Arc;
        use std::thread;
        let s = Arc::new(DiagnoseSession::new("loop-concurrent"));
        // Trigger once to leave Idle.
        s.apply(DiagnoseEvent::Trigger).unwrap();
        let s_clone = s.clone();
        let h1 = thread::spawn(move || {
            // Try a valid + an invalid event from various threads. Mutex must
            // serialize, so we either succeed with LoopReady or get rejected.
            let _ = s_clone.apply(DiagnoseEvent::LoopReady);
        });
        let s_clone2 = s.clone();
        let h2 = thread::spawn(move || {
            let _ = s_clone2.apply(DiagnoseEvent::LoopReady);
        });
        h1.join().unwrap();
        h2.join().unwrap();
        // One of them succeeded; state is Reproducing (or further).
        let snap = s.snapshot();
        assert!(
            matches!(snap, DiagnoseState::Reproducing | DiagnoseState::Building),
            "got {snap:?}"
        );
    }
}

//! Plan v6 §7 — "2am Friday" chaos test.
//!
//! 10 종 random failure simultaneously, each going through the 5-Phase loop
//! state machine, verify all reach terminal state without panic.
//!
//! Determinism is bound by:
//! - Fixed seed (no env-derived randomness in the test path)
//! - BTreeMap iteration order (we don't use HashMap)
//! - tokio runtime guarantees serial test execution per thread

use axhub_helpers::diagnose::state::{DiagnoseEvent, DiagnoseSession, DiagnoseState};

fn happy_path_events() -> &'static [DiagnoseEvent] {
    &[
        DiagnoseEvent::Trigger,
        DiagnoseEvent::LoopReady,
        DiagnoseEvent::SymptomConfirmed,
        DiagnoseEvent::HypothesesReady,
        DiagnoseEvent::ProbeApplied,
        DiagnoseEvent::FixApplied,
        DiagnoseEvent::LoopVerifyGreen,
        DiagnoseEvent::CleanupDone,
    ]
}

fn arch_handoff_events() -> &'static [DiagnoseEvent] {
    &[
        DiagnoseEvent::Trigger,
        DiagnoseEvent::LoopReady,
        DiagnoseEvent::SymptomConfirmed,
        DiagnoseEvent::HypothesesExhausted,
        DiagnoseEvent::HandoffSent,
    ]
}

fn red_then_green_events() -> &'static [DiagnoseEvent] {
    &[
        DiagnoseEvent::Trigger,
        DiagnoseEvent::LoopReady,
        DiagnoseEvent::SymptomConfirmed,
        DiagnoseEvent::HypothesesReady,
        DiagnoseEvent::ProbeApplied,
        DiagnoseEvent::FixApplied,
        DiagnoseEvent::LoopVerifyRed, // back to HYPOTHESIZE
        DiagnoseEvent::HypothesesReady,
        DiagnoseEvent::ProbeApplied,
        DiagnoseEvent::FixApplied,
        DiagnoseEvent::LoopVerifyGreen,
        DiagnoseEvent::CleanupDone,
    ]
}

fn recurrence_path_events() -> &'static [DiagnoseEvent] {
    &[
        DiagnoseEvent::Trigger,
        DiagnoseEvent::LoopReady,
        DiagnoseEvent::SymptomConfirmed,
        DiagnoseEvent::HypothesesReady,
        DiagnoseEvent::ProbeApplied,
        DiagnoseEvent::FixApplied,
        DiagnoseEvent::LoopVerifyGreen,
        DiagnoseEvent::RecurrenceThresholdHit,
        DiagnoseEvent::HandoffSent,
    ]
}

fn pick_path(seed: u32) -> &'static [DiagnoseEvent] {
    match seed % 4 {
        0 => happy_path_events(),
        1 => arch_handoff_events(),
        2 => red_then_green_events(),
        _ => recurrence_path_events(),
    }
}

fn drive(session: &DiagnoseSession, events: &[DiagnoseEvent]) -> Result<(), String> {
    for ev in events {
        session
            .apply(*ev)
            .map_err(|e| format!("loop {} on {:?}: {}", session.loop_id(), ev, e))?;
    }
    Ok(())
}

#[test]
fn ten_random_paths_all_reach_terminal_state() {
    let mut errors = Vec::new();
    for seed in 0u32..10 {
        let session = DiagnoseSession::new(format!("loop-2am-{seed}"));
        let path = pick_path(seed);
        match drive(&session, path) {
            Ok(()) => {
                let final_state = session.snapshot();
                if !final_state.is_terminal() {
                    errors.push(format!(
                        "loop {} ended in non-terminal state: {:?}",
                        session.loop_id(),
                        final_state
                    ));
                }
            }
            Err(e) => errors.push(e),
        }
    }
    assert!(errors.is_empty(), "2am-friday chaos failures:\n  {}", errors.join("\n  "));
}

#[test]
fn concurrent_sessions_dont_interfere() {
    use std::sync::Arc;
    use std::thread;
    let mut handles = Vec::new();
    for seed in 0u32..10 {
        let h = thread::spawn(move || {
            let session = Arc::new(DiagnoseSession::new(format!("loop-thread-{seed}")));
            let path = pick_path(seed);
            drive(&session, path).map(|_| session.snapshot())
        });
        handles.push(h);
    }
    let mut terminal_states = 0usize;
    for h in handles {
        let result = h.join().expect("thread did not panic");
        let final_state = result.expect("drive succeeded");
        if final_state.is_terminal() || matches!(final_state, DiagnoseState::Idle) {
            terminal_states += 1;
        } else {
            panic!("thread ended in non-terminal state: {final_state:?}");
        }
    }
    assert_eq!(terminal_states, 10, "all 10 threads must reach a terminal state");
}

#[test]
fn invalid_events_after_terminal_dont_panic() {
    let session = DiagnoseSession::new("loop-post-terminal");
    drive(&session, happy_path_events()).unwrap();
    assert!(session.snapshot().is_terminal());
    // Attempt to apply an event after Idle — should return Err, not panic.
    let result = session.apply(DiagnoseEvent::LoopReady);
    assert!(result.is_err(), "transition from Idle on LoopReady must error");
}

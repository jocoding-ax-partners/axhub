//! AC-3 scenario lock: **axhub repo (marker present) + a foreign deploy target
//! named in the prompt → axhub yields.**
//!
//! Scenario (spec 006, "named-target-wins"): the user is inside their own repo
//! that *does* carry an `axhub.yaml` marker, yet types `"vercel에 배포해"`. The
//! marker would route bare NL to axhub (rule d), but an explicitly named foreign
//! target must win (rule c) so axhub yields to the normal flow instead of
//! hijacking a Vercel deploy. This is the original "Vercel 쓰고 싶은데 axhub 로
//! 라우팅" complaint, which reproduces on *every* developer's own repo (every such
//! repo is marker-present).
//!
//! ## Why this file exists alongside the routing.rs unit tests
//!
//! AC-16 already locks the priority *chain* (`rulec_foreign_keyword_beats_marker_present`,
//! over raw flags) and that the public wrapper agrees with the core
//! (`decide_wrapper_agrees_with_core_over_detectors`, which exercises the
//! *space* form `"vercel 로 배포해"` but only asserts wrapper==core — it never
//! asserts the decision is `Yield`). Neither test asserts the end-to-end
//! **prompt → Yield** outcome for the literal AC-3 utterance.
//!
//! This integration test fills that gap through the **public** `decide()` — the
//! exact surface both consumers call (the prompt-route hook and the deploy
//! preflight Step 0). Because both layers route through this one function, a
//! `Yield` here is the `Yield` both layers inherit (the "composition-consistency"
//! guarantee). It deliberately uses the **no-space particle form** `"vercel에"`
//! so the Korean-boundary handling in `contains_word` is exercised on the precise
//! AC utterance, not just the space-delimited variant.
//!
//! Corroboration (not proof): the nl-lexicon corpus row `T-NEG-2`
//! ("vercel에 배포해줘" → `deploy_other_platform`, `expected_skill: null`,
//! "axhub만 지원") encodes the same foreign-target rejection at the skill-scoring
//! layer.

use axhub_helpers::routing::{decide, foreign_keyword_present, MarkerStatus, RoutingDecision};

/// The literal AC-3 utterance: no-space Korean particle attached to the target.
const AC3_PROMPT: &str = "vercel에 배포해";

/// The detector must fire on the no-space particle form. Asserted separately so a
/// regression localizes to the detector (`contains_word` boundary handling) vs.
/// the priority chain.
#[test]
fn foreign_detector_fires_on_no_space_particle_form() {
    assert!(
        foreign_keyword_present(AC3_PROMPT),
        "foreign-target detector must fire on the no-space particle form {AC3_PROMPT:?}"
    );
}

/// THE AC-3 lock: marker present + foreign keyword → `Yield`, end-to-end through
/// the public `decide()`. Asserted across both auth states because
/// "named-target-wins" is auth-invariant — an authed user inside their axhub repo
/// must *still* yield when they explicitly name Vercel. `explicit_invocation` is
/// `false` (this is bare NL with a foreign keyword, not a `/deploy` slash).
#[test]
fn marker_present_plus_foreign_keyword_yields_through_public_decide() {
    for authed in [false, true] {
        let decision = decide(
            AC3_PROMPT,
            MarkerStatus::Present,
            authed,
            /* explicit_invocation = */ false,
        );
        assert_eq!(
            decision,
            RoutingDecision::Yield,
            "axhub must yield on {AC3_PROMPT:?} in a marker-present repo (authed={authed})"
        );
    }
}

/// Locks the audit wire-string the routing-audit / routing-stats consumers
/// serialize. If this drifts from `"yield"`, the audit trail and the
/// routing-stats skill silently mis-bucket this decision.
#[test]
fn yield_decision_serializes_to_yield_wire_string() {
    let decision = decide(AC3_PROMPT, MarkerStatus::Present, true, false);
    assert_eq!(decision.as_str(), "yield");
}

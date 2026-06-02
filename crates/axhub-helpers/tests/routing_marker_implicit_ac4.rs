//! spec 006 AC 4 (rule **d**) — axhub repo + implicit `"배포해"` (marker present)
//! → axhub routing **activates**.
//!
//! Scenario (spec 006 §40, seed AC "axhub repo + 배포해 (implicit + marker) →
//! axhub routing activates"): a developer's own repo carries an `axhub.yaml`
//! marker, and the user types a bare natural-language deploy request in Korean
//! with **no** explicit target keyword and **no** slash command. The shared
//! routing decision must resolve to [`RoutingDecision::Axhub`] so the existing
//! `axhub.yaml` user keeps their implicit-deploy behavior (backward-compat).
//!
//! This exercises the *discriminating* axis of AC 4 — the prompt — through the
//! public [`decide`] wrapper, which runs the real keyword detectors. The
//! session-start eager-infra gate is deliberately NOT used here: it is
//! prompt-blind (`should_run_eager_infra` hardcodes bare-NL flags), so a green
//! result there proves AC 7's session gating, not AC 4's prompt routing.
//!
//! These assertions live at the `decide()` level by necessity: there is no
//! prompt-bearing `route`/`decide` helper subcommand today (only the
//! prompt-blind `session-eager-gate`), so the binary cannot host a faithful
//! prompt→decision end-to-end check. The decision function plus the in-crate
//! 48-combo matrix (`routing::tests`) is the correct ceiling for AC 4 right now.

use axhub_helpers::routing::{
    axhub_keyword_present, decide, foreign_keyword_present, is_slash_invocation, MarkerStatus,
    RoutingDecision,
};

/// Korean implicit deploy prompts: bare NL, no explicit `"axhub"`, no foreign
/// target keyword, no slash. These are exactly the rule-d inputs.
const IMPLICIT_KOREAN_DEPLOY_PROMPTS: &[&str] =
    &["배포해", "배포해줘", "그거 배포해줘", "지금 배포해"];

/// The premises of rule d for every sample prompt: it is genuinely *bare NL*.
/// If any of these flipped, the prompt would be routed by a higher-priority
/// rule (0/a/b/c) and the test below would be asserting the wrong thing — so
/// these guards document *why* `"배포해"` follows the marker rather than a keyword.
#[test]
fn implicit_korean_prompts_are_genuinely_bare_nl() {
    for &prompt in IMPLICIT_KOREAN_DEPLOY_PROMPTS {
        assert!(
            !axhub_keyword_present(prompt),
            "{prompt:?} must NOT carry the explicit 'axhub' keyword (else rule b, not d)"
        );
        assert!(
            !foreign_keyword_present(prompt),
            "{prompt:?} must NOT carry a foreign target keyword (else rule c yields)"
        );
        assert!(
            !is_slash_invocation(prompt),
            "{prompt:?} must NOT be a slash invocation (else rule 0)"
        );
    }
}

/// AC 4 core: marker **Present** + implicit `"배포해"` → `Axhub`, regardless of
/// auth (rule d is auth-independent; the auth-conditional fallback only applies
/// to the `Unknown` marker error path, spec §99). Asserted across `authed` so it
/// is the *marker*, not an incidental auth state, that activates axhub routing.
#[test]
fn marker_present_implicit_korean_deploy_routes_to_axhub() {
    for &prompt in IMPLICIT_KOREAN_DEPLOY_PROMPTS {
        for &authed in &[false, true] {
            let decision = decide(
                prompt,
                MarkerStatus::Present,
                authed,
                /* explicit */ false,
            );
            assert_eq!(
                decision,
                RoutingDecision::Axhub,
                "AC 4: marker-present implicit {prompt:?} (authed={authed}) must activate axhub routing"
            );
        }
    }
}

/// Control proving the *marker* is what activates axhub for an implicit prompt:
/// the identical bare-NL prompt with the marker **Absent** does NOT route to
/// axhub (rule e → `Ignore`). This isolates AC 4's "+ marker" clause — without
/// the marker, the same `"배포해"` is a zero-footprint pass-through. (The full
/// non-marker→ignore behavior is a sibling AC; here it is only the negative
/// control that gives AC 4's positive assertion its meaning.)
#[test]
fn same_implicit_prompt_without_marker_does_not_activate_axhub() {
    for &prompt in IMPLICIT_KOREAN_DEPLOY_PROMPTS {
        for &authed in &[false, true] {
            let decision = decide(
                prompt,
                MarkerStatus::Absent,
                authed,
                /* explicit */ false,
            );
            assert_ne!(
                decision,
                RoutingDecision::Axhub,
                "without a marker, implicit {prompt:?} must not activate axhub (rule e)"
            );
            assert_eq!(
                decision,
                RoutingDecision::Ignore,
                "bare-NL + no marker is rule e → Ignore"
            );
        }
    }
}

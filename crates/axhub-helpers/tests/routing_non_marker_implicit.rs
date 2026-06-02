//! spec 006 (AC 1) — non-marker repo + implicit "배포해" → `Ignore`.
//!
//! The canonical zero-footprint row of the routing matrix: a developer's own
//! repo that has **no** `axhub.yaml` marker, where the user types a bare
//! natural-language deploy request ("배포해") with no `axhub` keyword, no
//! foreign target, and no slash invocation. axhub must NOT claim the prompt —
//! the shared decision is [`RoutingDecision::Ignore`] (rule `e`), so the hook
//! stays silent and the deploy preflight disambiguates instead of routing to
//! axhub.
//!
//! This test fills the gap the routing unit tests leave open by composing the
//! THREE real inputs the AC names, end to end through the public API:
//!   1. marker walk-up over a real filesystem → [`MarkerStatus::Absent`],
//!   2. prompt detection on the literal "배포해" → bare NL (no keywords), and
//!   3. [`decide`] over those derived inputs → [`RoutingDecision::Ignore`].
//!
//! The crate's in-module tests cover (2) and (3) only in isolation:
//! `decide_wrapper_agrees_with_core_over_detectors` feeds "배포해" but asserts
//! only wrapper==core (never the literal `Ignore`), and
//! `ruled_rulee_bare_nl_follows_marker` asserts `Absent → Ignore` from raw
//! `(false, false, …)` booleans — neither derives the decision from the prompt
//! AND a filesystem walk-up together. That composition is exactly AC 1.
//!
//! Scope note: silencing the `ignore` decision at the hook layer (and the
//! once-per-project grace message) is owned by the separate
//! `hook-integration-complete` exit condition, not this AC. This test pins the
//! decision-boundary contract, which is the stable source of truth both the
//! hook and the deploy preflight consume.

use axhub_helpers::routing::{
    axhub_keyword_present, decide, find_marker_from, foreign_keyword_present, MarkerStatus,
    RoutingDecision,
};

/// The literal implicit deploy prompt from AC 1 ("deploy", Korean imperative).
const IMPLICIT_DEPLOY_PROMPT: &str = "배포해";

/// Build a non-marker git repo in a fresh tempdir: a `.git` directory (so the
/// walk-up terminates deterministically at the repo root and cannot escape to
/// ancestors) and deliberately NO `axhub.yaml`. Mirrors the crate's own
/// `marker_absent_stops_at_git_root` setup.
fn non_marker_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    tmp
}

/// The walk-up over a `.git` repo with no `axhub.yaml` resolves to `Absent`
/// (a confirmed "no marker", not an `Unknown` error). This is the marker half
/// of AC 1 — established independently so the decision assertion below rests on
/// a verified input, not an assumption.
#[test]
fn non_marker_repo_walk_up_is_absent() {
    let repo = non_marker_repo();
    let nested = repo.path().join("pkg").join("src");
    std::fs::create_dir_all(&nested).expect("mkdir nested");

    assert_eq!(
        find_marker_from(&nested),
        MarkerStatus::Absent,
        "a .git repo without axhub.yaml must resolve to Absent, not Unknown"
    );
}

/// "배포해" is bare natural language: it names neither axhub nor any foreign
/// target, so it falls through every keyword rule (0/a/b/c) to the
/// marker-driven arm. Documents *why* the decision below is governed by the
/// (absent) marker rather than a keyword.
#[test]
fn implicit_deploy_prompt_is_bare_natural_language() {
    assert!(
        !axhub_keyword_present(IMPLICIT_DEPLOY_PROMPT),
        "\"배포해\" must not be read as an explicit axhub keyword"
    );
    assert!(
        !foreign_keyword_present(IMPLICIT_DEPLOY_PROMPT),
        "\"배포해\" must not be read as naming a foreign target"
    );
}

/// The AC 1 composition: non-marker walk-up (`Absent`) + implicit "배포해"
/// (bare NL) + no slash → [`RoutingDecision::Ignore`], asserted for BOTH auth
/// states. Auth does not change the outcome here: with the marker confirmed
/// `Absent` (rule `e`, not the `Unknown` error fallback), auth never enters the
/// decision — so an authed developer in a non-axhub repo is ignored exactly
/// like an unauthed one. That auth-independence is the substance of "no axhub
/// routing": axhub yields the repo to its pass-through default regardless of
/// whether the user happens to be logged in.
#[test]
fn non_marker_implicit_deploy_is_ignored_regardless_of_auth() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");

    for authed in [false, true] {
        let decision = decide(
            IMPLICIT_DEPLOY_PROMPT,
            marker,
            authed,
            /* explicit_invocation = */ false,
        );
        assert_eq!(
            decision,
            RoutingDecision::Ignore,
            "non-marker repo + implicit \"배포해\" must Ignore (authed={authed})"
        );
        // The wire form the routing-audit jsonl records for this row.
        assert_eq!(decision.as_str(), "ignore");
    }
}

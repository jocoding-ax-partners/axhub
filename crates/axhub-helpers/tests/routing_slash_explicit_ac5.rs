//! spec 006 AC 5 (rule **0**) — `/deploy` or `/axhub:deploy` slash command →
//! **always** axhub, regardless of marker or keywords.
//!
//! Scenario (spec 006 §36 "slash invocation → axhub (marker·keyword 무관, 최강
//! explicit — conflict 보다 위)", §54, §72, §88 "`axhub.yaml` 없음 + `/deploy` →
//! explicit → axhub"): the user runs an explicit axhub slash command. This is the
//! strongest explicit signal, so the shared decision must resolve to
//! [`RoutingDecision::Axhub`] above *every* other rule — even when a foreign
//! target is named in the same prompt (rule c), even when both axhub and a
//! foreign target are named (rule a), even with no marker, and even for an
//! unauthed user where the bare-NL error fallback would otherwise `Ignore`.
//!
//! ## Why this file exists alongside the routing.rs unit tests
//!
//! The crate's `rule0_slash_invocation_always_wins` locks the *priority* — but it
//! hard-codes `explicit_invocation = true`. It never derives that flag from a real
//! `/deploy` utterance, and `slash_invocation_detection` checks the detector in
//! isolation. Neither composes **prompt → `is_slash_invocation` → `decide`**, which
//! is exactly how the hook feeds rule 0 (`is_slash_invocation` doc: "the hook
//! detects it from the prompt text … both feed the result into [`decide`] as
//! `explicit_invocation`"). This file makes that derivation the spine: every
//! assertion runs `decide(prompt, …, is_slash_invocation(prompt))` through the
//! **public** surface both consumers call, so a green here is the `Axhub` both the
//! prompt-route hook and the deploy preflight Step 0 inherit
//! (composition-consistency).
//!
//! ## Isolating rule 0 from rules b/c/a (a trap the naive test falls into)
//!
//! `/axhub:deploy` *alone* is already `Axhub` via **rule b** — it contains the
//! literal "axhub", so `axhub_keyword_present` fires too. Asserting it equals
//! `Axhub` therefore does NOT prove rule 0 did the work. To genuinely isolate the
//! slash, the conflict tests below carry a **foreign** keyword and use an
//! explicit-vs-implicit control on the *same* text: with the slash suppressed the
//! decision is `Yield`/`Ask`, and only the slash flips it to `Axhub`.
//!
//! Scope note: silencing/acting on the decision at the hook layer and the deploy
//! preflight Step 0 is owned by the separate `hook-integration-complete` /
//! `preflight-integration-complete` exit conditions, not this AC. And per the
//! audit sibling, "explicit" is an audit input *label*, not a 5th decision enum
//! variant — a slash decision is `Axhub` and serializes to `"axhub"`. This test
//! pins the decision-boundary contract those layers consume.

use axhub_helpers::routing::{
    axhub_keyword_present, decide, foreign_keyword_present, is_slash_invocation, MarkerStatus,
    RoutingDecision,
};

/// The literal AC 5 slash utterances (both forms named in the AC) plus a
/// leading-whitespace / trailing-arg variant to exercise `trim_start` + prefix
/// matching on the precise shapes a user types.
const SLASH_UTTERANCES: &[&str] = &["/deploy", "/axhub:deploy", "  /deploy to prod"];

/// Every marker state, so the core assertion proves rule 0 dominates the marker
/// axis end to end: `Present` (would be Axhub anyway), `Absent` (spec §88, would
/// be `Ignore` as bare NL), and `Unknown` (the fs-error fallback, which for an
/// unauthed user would be `Ignore`).
const ALL_MARKERS: &[MarkerStatus] = &[
    MarkerStatus::Present,
    MarkerStatus::Absent,
    MarkerStatus::Unknown,
];

/// Premise guard: the detector fires on the AC utterances and stays quiet on the
/// bare-NL look-alikes. If this regresses, a failure localizes to
/// `is_slash_invocation` rather than the priority chain.
#[test]
fn slash_detector_fires_only_on_slash_utterances() {
    for &prompt in SLASH_UTTERANCES {
        assert!(
            is_slash_invocation(prompt),
            "{prompt:?} must be detected as an explicit slash invocation"
        );
    }
    for &not_slash in &["deploy", "배포해", "please /deploy", "axhub deploy"] {
        assert!(
            !is_slash_invocation(not_slash),
            "{not_slash:?} must NOT be read as a slash invocation"
        );
    }
}

/// AC 5 core: a slash utterance routes to `Axhub` for **every** marker state and
/// **both** auth states, with `explicit_invocation` *derived from the prompt*
/// (the hook's real path) rather than hard-coded. This single matrix covers the
/// spec's "marker·keyword 무관" claim on the marker/auth axes — including the two
/// rows the marker would otherwise lose: `Absent` (spec §88) and `Unknown` +
/// unauthed (where the err fallback would `Ignore`).
#[test]
fn slash_invocation_routes_axhub_across_every_marker_and_auth() {
    for &prompt in SLASH_UTTERANCES {
        let explicit = is_slash_invocation(prompt);
        for &marker in ALL_MARKERS {
            for &authed in &[false, true] {
                let decision = decide(prompt, marker, authed, explicit);
                assert_eq!(
                    decision,
                    RoutingDecision::Axhub,
                    "AC 5: slash {prompt:?} must route axhub (marker={marker:?} authed={authed})"
                );
                // Wire form the routing-audit jsonl / routing-stats consumers read.
                assert_eq!(decision.as_str(), "axhub");
            }
        }
    }
}

/// Rule **0 > c**: a slash prompt that *also* names a foreign target. As bare NL
/// the foreign keyword wins (`Yield`), so this is the precise prompt where the
/// slash has to override "named-target-wins". The explicit-vs-implicit control on
/// the *same* text is what proves the slash — not some other input — does the work.
#[test]
fn slash_beats_foreign_keyword() {
    const FOREIGN_SLASH: &str = "/deploy to vercel";
    // Premises: it is genuinely both a slash AND a foreign-target prompt.
    assert!(is_slash_invocation(FOREIGN_SLASH));
    assert!(foreign_keyword_present(FOREIGN_SLASH));
    assert!(!axhub_keyword_present(FOREIGN_SLASH));

    for &marker in ALL_MARKERS {
        for &authed in &[false, true] {
            // Control: suppress the slash → rule c (named-target-wins) yields.
            assert_eq!(
                decide(FOREIGN_SLASH, marker, authed, /* explicit */ false),
                RoutingDecision::Yield,
                "control: {FOREIGN_SLASH:?} as bare NL must yield to the foreign target"
            );
            // Rule 0: the slash overrides the foreign target → axhub.
            assert_eq!(
                decide(
                    FOREIGN_SLASH,
                    marker,
                    authed,
                    is_slash_invocation(FOREIGN_SLASH)
                ),
                RoutingDecision::Axhub,
                "rule 0 must beat rule c on {FOREIGN_SLASH:?} (marker={marker:?} authed={authed})"
            );
        }
    }
}

/// Rule **0 > a**: a slash prompt naming *both* axhub and a foreign target. As
/// bare NL that is the ambiguous case (`Ask`); the slash must collapse the
/// ambiguity straight to `Axhub`. Again the explicit-vs-implicit control on the
/// same text isolates the slash as the deciding input.
#[test]
fn slash_beats_axhub_plus_foreign_ambiguity() {
    const BOTH_SLASH: &str = "/axhub:deploy to vercel";
    // Premises: slash AND axhub-keyword AND foreign-keyword all present.
    assert!(is_slash_invocation(BOTH_SLASH));
    assert!(axhub_keyword_present(BOTH_SLASH));
    assert!(foreign_keyword_present(BOTH_SLASH));

    for &marker in ALL_MARKERS {
        for &authed in &[false, true] {
            // Control: suppress the slash → rule a (axhub + foreign) asks.
            assert_eq!(
                decide(BOTH_SLASH, marker, authed, /* explicit */ false),
                RoutingDecision::Ask,
                "control: {BOTH_SLASH:?} as bare NL is the ambiguous (Ask) case"
            );
            // Rule 0: the slash resolves the ambiguity → axhub.
            assert_eq!(
                decide(BOTH_SLASH, marker, authed, is_slash_invocation(BOTH_SLASH)),
                RoutingDecision::Axhub,
                "rule 0 must beat rule a on {BOTH_SLASH:?} (marker={marker:?} authed={authed})"
            );
        }
    }
}

/// spec §88 end-to-end over a **real filesystem**: a non-marker repo (`.git`, no
/// `axhub.yaml`) where an unauthed user runs `/deploy`. The marker walk-up
/// resolves to `Absent` and the user is logged out — the worst case for the
/// marker/auth axes — yet the slash still routes to `Axhub`. The in-test control
/// (same prompt, slash suppressed → `Ignore`) proves the slash is the sole reason
/// the zero-footprint pass-through is overridden here.
#[test]
fn non_marker_unauthed_slash_still_routes_axhub() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    let nested = tmp.path().join("pkg").join("src");
    std::fs::create_dir_all(&nested).expect("mkdir nested");

    let marker = axhub_helpers::routing::find_marker_from(&nested);
    assert_eq!(
        marker,
        MarkerStatus::Absent,
        "precondition: a .git repo without axhub.yaml resolves to Absent"
    );

    const SLASH: &str = "/deploy";
    // Control: without the slash, this is bare NL + Absent marker → Ignore
    // (the zero-footprint pass-through an unauthed non-marker user gets).
    assert_eq!(
        decide(SLASH, marker, /* authed */ false, /* explicit */ false),
        RoutingDecision::Ignore,
        "control: bare NL in a non-marker repo must Ignore"
    );
    // Rule 0: the slash overrides marker-absence AND the unauthed state → axhub.
    assert_eq!(
        decide(SLASH, marker, /* authed */ false, is_slash_invocation(SLASH)),
        RoutingDecision::Axhub,
        "spec §88: `axhub.yaml` 없음 + /deploy must still route axhub even unauthed"
    );
}

//! spec 006 AC 14 — the **marker × keyword × slash × prompt** combination matrix.
//!
//! This is the end-to-end "all branches green" test the `test-matrix-passes`
//! exit condition names: every priority rule (`0`,`a`,`b`,`c`,`d`,`e`,`err`)
//! plus the fail-open auth-conditional path, exercised through the **public**
//! [`decide`] surface that both consumers (the prompt-route hook and the deploy
//! preflight Step 0) call — so a green here is the decision both layers inherit
//! (composition-consistency).
//!
//! ## How this differs from the existing routing.rs matrix (anti-tautology)
//!
//! `routing.rs::decide_from_flags_matches_reference_for_all_inputs` is a
//! **pure-flag** matrix: it feeds raw `axhub`/`foreign` booleans into
//! `decide_from_flags` and compares against a reference chain. It never touches a
//! real prompt, so it cannot catch a keyword-detector regression.
//! `decide_wrapper_agrees_with_core_over_detectors` *does* use prompts, but only
//! asserts the wrapper agrees with the core fed the **same detector output** — a
//! consistency claim, not an absolute one. If both `decide` and the detectors
//! drifted together, that test would still pass.
//!
//! This file closes both gaps. Each prompt carries a **hand-annotated** keyword
//! class (`AXHUB`/`FOREIGN`/`SLASH`). Two independent checks run per prompt:
//!
//!   1. **Premise guard** — the live detectors must agree with the annotation
//!      (`axhub_keyword_present`/`foreign_keyword_present`/`is_slash_invocation`).
//!      A detector regression localizes here, not in the priority chain.
//!   2. **Oracle check** — the **annotations** (not detector output) feed an
//!      independent [`oracle`] reimplementation of the spec priority table, and
//!      its verdict is compared against `decide(prompt, …)`. Because the oracle
//!      is fed hand-written truth rather than the production detectors, a joint
//!      detector+chain drift is caught (the wrapper test's blind spot).
//!
//! ## Coverage proof
//!
//! The matrix asserts — via a collected rule-id set — that **all seven** priority
//! branches actually fire over the corpus. A corpus that silently stopped
//! exercising, say, the `err` branch would fail the coverage assertion even if
//! every individual cell still matched.

use std::collections::BTreeSet;

use axhub_helpers::routing::{
    axhub_keyword_present, decide, find_marker_from, foreign_keyword_present, is_slash_invocation,
    MarkerStatus, RoutingDecision,
};

/// One prompt plus its hand-annotated keyword class. The annotations are the
/// independent truth the oracle consumes; the premise guard pins the live
/// detectors to them so the two stay honest.
struct Case {
    prompt: &'static str,
    /// Expected `axhub_keyword_present(prompt)`.
    axhub: bool,
    /// Expected `foreign_keyword_present(prompt)`.
    foreign: bool,
    /// Expected `is_slash_invocation(prompt)`.
    slash: bool,
}

/// The prompt corpus, one row per `(axhub, foreign, slash)` class the spec
/// distinguishes. Korean + English variants so the whole-word detector is
/// exercised across both scripts (e.g. `"vercel로"` vs `"to vercel"`).
const CASES: &[Case] = &[
    // ── bare NL (no keyword, no slash) — drives rules d/e/err via the marker ──
    Case {
        prompt: "배포해",
        axhub: false,
        foreign: false,
        slash: false,
    },
    Case {
        prompt: "그냥 빌드만 해줘",
        axhub: false,
        foreign: false,
        slash: false,
    },
    Case {
        prompt: "deploy this app for me",
        axhub: false,
        foreign: false,
        slash: false,
    },
    // ── axhub-only (rule b: marker-independent Axhub) ──
    Case {
        prompt: "axhub 으로 배포해",
        axhub: true,
        foreign: false,
        slash: false,
    },
    Case {
        prompt: "use axhub to ship it",
        axhub: true,
        foreign: false,
        slash: false,
    },
    Case {
        prompt: "read axhub.yaml first",
        axhub: true,
        foreign: false,
        slash: false,
    }, // bounded by '.'
    // ── foreign-only (rule c: named-target-wins → Yield, beats marker) ──
    Case {
        prompt: "vercel 로 배포해",
        axhub: false,
        foreign: true,
        slash: false,
    },
    Case {
        prompt: "push to netlify",
        axhub: false,
        foreign: true,
        slash: false,
    },
    Case {
        prompt: "deploy on fly.io",
        axhub: false,
        foreign: true,
        slash: false,
    }, // bounded by '.'
    Case {
        prompt: "cloudflare 에 올려줘",
        axhub: false,
        foreign: true,
        slash: false,
    },
    Case {
        prompt: "ship to render now",
        axhub: false,
        foreign: true,
        slash: false,
    },
    Case {
        prompt: "railway up please",
        axhub: false,
        foreign: true,
        slash: false,
    },
    // ── axhub + foreign (rule a: ambiguous → Ask) ──
    Case {
        prompt: "axhub 말고 vercel 로",
        axhub: true,
        foreign: true,
        slash: false,
    },
    Case {
        prompt: "deploy to axhub or netlify",
        axhub: true,
        foreign: true,
        slash: false,
    },
    // ── slash-text prompts. The slash axis is swept as an independent boolean
    //    below, so these also act as rule-c / rule-a "controls" when explicit is
    //    suppressed (the AC5 technique), and as rule-0 winners when it is set. ──
    Case {
        prompt: "/deploy",
        axhub: false,
        foreign: false,
        slash: true,
    },
    Case {
        prompt: "/배포 paydrop",
        axhub: false,
        foreign: false,
        slash: true,
    },
    Case {
        prompt: "/axhub:deploy",
        axhub: true,
        foreign: false,
        slash: true,
    }, // ':' bounds "axhub"
    Case {
        prompt: "/deploy to vercel",
        axhub: false,
        foreign: true,
        slash: true,
    },
    Case {
        prompt: "/axhub:deploy to vercel",
        axhub: true,
        foreign: true,
        slash: true,
    },
];

const ALL_MARKERS: &[MarkerStatus] = &[
    MarkerStatus::Present,
    MarkerStatus::Absent,
    MarkerStatus::Unknown,
];

/// Independent reference for the spec 006 priority table (§32-43 + `err`),
/// written deliberately unlike the production early-return chain (nested
/// matches). Consumes the **hand annotations** and the swept `explicit` flag —
/// never the production detectors — so agreement with [`decide`] is a real
/// cross-check, not a restatement.
fn oracle(
    axhub: bool,
    foreign: bool,
    marker: MarkerStatus,
    authed: bool,
    explicit: bool,
) -> RoutingDecision {
    match (explicit, axhub, foreign) {
        (true, _, _) => RoutingDecision::Axhub, // rule 0 — slash wins outright
        (false, true, true) => RoutingDecision::Ask, // rule a
        (false, true, false) => RoutingDecision::Axhub, // rule b
        (false, false, true) => RoutingDecision::Yield, // rule c
        (false, false, false) => match (marker, authed) {
            (MarkerStatus::Present, _) => RoutingDecision::Axhub, // rule d
            (MarkerStatus::Absent, _) => RoutingDecision::Ignore, // rule e
            (MarkerStatus::Unknown, true) => RoutingDecision::Axhub, // err (authed)
            (MarkerStatus::Unknown, false) => RoutingDecision::Ignore, // err (unauthed)
        },
    }
}

/// Stable id for the priority branch a given input row resolves through, used to
/// prove the corpus exercises every branch. Coverage is a weaker claim than
/// correctness, so sharing precedence shape with [`oracle`] is acceptable here.
fn rule_id(axhub: bool, foreign: bool, marker: MarkerStatus, explicit: bool) -> &'static str {
    if explicit {
        "0"
    } else if axhub && foreign {
        "a"
    } else if axhub {
        "b"
    } else if foreign {
        "c"
    } else {
        match marker {
            MarkerStatus::Present => "d",
            MarkerStatus::Absent => "e",
            MarkerStatus::Unknown => "err",
        }
    }
}

/// Premise guard: the live detectors agree with every hand annotation. If this
/// regresses, a keyword/slash misclassification is the cause — isolated from the
/// priority-chain assertions below.
#[test]
fn detectors_match_annotations() {
    for c in CASES {
        assert_eq!(
            axhub_keyword_present(c.prompt),
            c.axhub,
            "axhub_keyword_present({:?}) annotation mismatch",
            c.prompt
        );
        assert_eq!(
            foreign_keyword_present(c.prompt),
            c.foreign,
            "foreign_keyword_present({:?}) annotation mismatch",
            c.prompt
        );
        assert_eq!(
            is_slash_invocation(c.prompt),
            c.slash,
            "is_slash_invocation({:?}) annotation mismatch",
            c.prompt
        );
    }
}

/// THE matrix: prompt-corpus × marker(3) × auth(2) × slash(2). Every cell's
/// `decide(prompt, …)` is compared against the independent [`oracle`] fed the
/// hand annotations, and the set of branches reached is asserted to be the full
/// `{0,a,b,c,d,e,err}`.
#[test]
fn combination_matrix_matches_oracle_and_covers_all_branches() {
    let mut covered: BTreeSet<&'static str> = BTreeSet::new();
    let mut cells = 0usize;

    for c in CASES {
        for &marker in ALL_MARKERS {
            for &authed in &[false, true] {
                // Slash swept as an independent boolean (the same value feeds
                // both decide() and the oracle — never recomputed inside the
                // oracle). explicit=false on a slash-text prompt is the AC5
                // "control": the keywords decide; only explicit=true triggers
                // rule 0.
                for &explicit in &[false, true] {
                    let got = decide(c.prompt, marker, authed, explicit);
                    let want = oracle(c.axhub, c.foreign, marker, authed, explicit);
                    assert_eq!(
                        got, want,
                        "matrix drift on prompt={:?} marker={:?} authed={} explicit={}",
                        c.prompt, marker, authed, explicit
                    );
                    covered.insert(rule_id(c.axhub, c.foreign, marker, explicit));
                    cells += 1;
                }
            }
        }
    }

    let expected_branches: BTreeSet<&'static str> =
        ["0", "a", "b", "c", "d", "e", "err"].into_iter().collect();
    assert_eq!(
        covered,
        expected_branches,
        "matrix must exercise every priority branch; missing {:?}",
        expected_branches.difference(&covered).collect::<Vec<_>>()
    );
    // 19 prompts × 3 markers × 2 auth × 2 slash = 228 cells. A guard that the
    // loop ran fully (no corpus truncation).
    assert_eq!(cells, CASES.len() * 3 * 2 * 2);
    assert_eq!(cells, 228);
}

/// Spotlight on the precise drift the routing rework exists to kill (spec §59):
/// a foreign target named in a **marker-present** repo must `Yield` (rule c) —
/// it must NOT route to `Axhub` off the marker (rule d). Asserted via the public
/// surface, over both auth states, with explicit suppressed.
#[test]
fn foreign_keyword_yields_even_with_marker_present() {
    for c in CASES.iter().filter(|c| c.foreign && !c.axhub && !c.slash) {
        for &authed in &[false, true] {
            assert_eq!(
                decide(
                    c.prompt,
                    MarkerStatus::Present,
                    authed,
                    /* explicit */ false
                ),
                RoutingDecision::Yield,
                "named-target-wins must hold for {:?} on a marker-present repo (authed={authed})",
                c.prompt
            );
        }
    }
}

/// The fail-open auth-conditional path the exit condition calls out explicitly:
/// bare NL + `Unknown` marker (a walk-up error) routes `Axhub` when authed and
/// `Ignore` when not — keeping authed users' "배포해"→axhub behavior through
/// transient fs errors while unauthed users stay zero-footprint.
#[test]
fn bare_nl_unknown_marker_is_auth_conditional() {
    for c in CASES.iter().filter(|c| !c.foreign && !c.axhub && !c.slash) {
        assert_eq!(
            decide(
                c.prompt,
                MarkerStatus::Unknown,
                /* authed */ true,
                false
            ),
            RoutingDecision::Axhub,
            "err branch: authed bare NL must fall open to axhub for {:?}",
            c.prompt
        );
        assert_eq!(
            decide(
                c.prompt,
                MarkerStatus::Unknown,
                /* authed */ false,
                false
            ),
            RoutingDecision::Ignore,
            "err branch: unauthed bare NL must stay zero-footprint for {:?}",
            c.prompt
        );
    }
}

/// Rule 0 via the **real hook derivation** (`explicit = is_slash_invocation`),
/// not a swept boolean: every slash-text prompt routes `Axhub` regardless of
/// marker/auth — the composition the prompt-route hook actually runs. The
/// suppressed-slash control on the same text proves the slash is the deciding
/// input (a foreign/ambiguous slash would otherwise Yield/Ask).
#[test]
fn slash_prompts_route_axhub_via_real_derivation() {
    for c in CASES.iter().filter(|c| c.slash) {
        let explicit = is_slash_invocation(c.prompt);
        assert!(explicit, "premise: {:?} is a slash invocation", c.prompt);
        for &marker in ALL_MARKERS {
            for &authed in &[false, true] {
                assert_eq!(
                    decide(c.prompt, marker, authed, explicit),
                    RoutingDecision::Axhub,
                    "rule 0: slash {:?} must route axhub (marker={marker:?} authed={authed})",
                    c.prompt
                );
                assert_eq!(decide(c.prompt, marker, authed, explicit).as_str(), "axhub");
            }
        }
    }
}

/// Ground the matrix's abstract `MarkerStatus` in the real cwd→git-root walk-up
/// for one Present and one Absent row, so the marker axis is not purely
/// synthetic. (AC5 owns the full walk-up scenario coverage; this is a single
/// sanity link from `find_marker_from` into `decide`.)
#[test]
fn fs_backed_marker_feeds_decide() {
    // Present: .git + axhub.yaml at root, nested cwd → bare NL routes Axhub (rule d).
    let present = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(present.path().join(".git")).expect("mkdir .git");
    std::fs::write(present.path().join("axhub.yaml"), "app: demo\n").expect("write marker");
    let nested = present.path().join("pkg").join("src");
    std::fs::create_dir_all(&nested).expect("mkdir nested");
    let marker = find_marker_from(&nested);
    assert_eq!(marker, MarkerStatus::Present);
    assert_eq!(
        decide("배포해", marker, /* authed */ false, false),
        RoutingDecision::Axhub,
        "rule d: bare NL in a real marker-present repo must route axhub"
    );

    // Absent: .git, no axhub.yaml → bare NL ignores (rule e, zero-footprint).
    let absent = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(absent.path().join(".git")).expect("mkdir .git");
    let nested2 = absent.path().join("pkg").join("src");
    std::fs::create_dir_all(&nested2).expect("mkdir nested");
    let marker2 = find_marker_from(&nested2);
    assert_eq!(marker2, MarkerStatus::Absent);
    assert_eq!(
        decide("배포해", marker2, /* authed */ true, false),
        RoutingDecision::Ignore,
        "rule e: bare NL in a real non-marker repo must ignore even when authed"
    );
}

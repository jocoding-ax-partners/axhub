//! spec 006 (AC 2) — non-marker repo + explicit `"axhub"` keyword ("axhub에
//! 배포해") routes to axhub via **lazy auth bootstrap**.
//!
//! ## What this AC pins (and how it relates to its siblings)
//!
//! AC 2 is the explicit-keyword half of the `explicit-always-works` evaluation
//! principle: "Explicit axhub invocations (/deploy, 'axhub' keyword) function
//! regardless of marker presence with lazy auth bootstrap." This file pins the
//! **decision-layer** contribution for the *keyword* modality only:
//!
//!   1. marker walk-up over a real non-marker filesystem → [`MarkerStatus`]
//!      (`Absent`, and the `Unknown` error fallback), and
//!   2. the explicit `"axhub"` keyword → [`RoutingDecision::Axhub`] from
//!      [`decide`], **marker-independent and at `authed = false`**.
//!
//! ### Why `authed = false` IS the "lazy auth bootstrap" claim at this layer
//!
//! AC 7 made `session-start` skip the eager infra chain (token-init / warmup /
//! quality-context) in repos with no `axhub.yaml`, so a non-marker session is
//! zero-footprint: the helper auth token-file is NOT pre-created. At decision
//! time that means `authed = false` (the token-file `.exists()` stat is false).
//! The substance of "via lazy auth bootstrap" *at the routing layer* is exactly
//! that the explicit keyword still routes to [`RoutingDecision::Axhub`] in this
//! no-token state — so control reaches the deploy path, where auth is then
//! resolved at call time. If routing instead required a pre-existing token, the
//! keyword path would silently die in every zero-footprint repo and lazy
//! bootstrap could never fire. The `authed = false → Axhub` row below is that
//! guarantee.
//!
//! ### Boundary with sibling ACs (intentional overlap, not a missed dedup)
//!
//! - The **call-time auth probe endpoint** (`run_preflight_with_runner` →
//!   `auth_ok` / `EXIT_OK` on success, `EXIT_AUTH` on failure) is owned and
//!   proven by **AC 9** (`routing_lazy_auth_explicit_ac9.rs`). This file does
//!   NOT re-assert the probe — re-owning it would be the very layer-drift the
//!   composition-consistency guarantee exists to prevent. AC 9 shares the
//!   identical `"axhub에 배포해"` prompt; AC 2 is its decision-layer corroboration
//!   scoped to the keyword modality + the `Unknown` marker increment AC 9 did
//!   not exercise for the keyword row.
//! - The **slash modality** (`/deploy`, `/axhub:deploy`) is owned by **AC 5**
//!   (`routing_slash_explicit_ac5.rs`). This file stays strictly on the
//!   `"axhub"` keyword (rule `b`), never touching `is_slash_invocation`.
//! - The **bare-NL ignore contrast** is owned by **AC 1**
//!   (`routing_non_marker_implicit.rs`). It is reused here only as a local
//!   non-vacuity discriminator (see below), not re-pinned as the AC's outcome.

use axhub_helpers::routing::{
    axhub_keyword_present, decide, find_marker_from, foreign_keyword_present, MarkerStatus,
    RoutingDecision,
};

/// The literal explicit-keyword prompt from AC 2 ("deploy to axhub", Korean).
const EXPLICIT_KEYWORD_PROMPT: &str = "axhub에 배포해";

/// A second realistic phrasing of the same explicit intent. Proves the routing
/// decision rests on the `"axhub"` keyword being present anywhere in the prompt,
/// not on one exact byte string.
const EXPLICIT_KEYWORD_PROMPT_ALT: &str = "이거 axhub 로 배포해줘";

/// The bare-NL deploy prompt (AC 1's literal). Used here ONLY as a discriminator:
/// in the *same* non-marker repo it must `Ignore`, proving the keyword is what
/// flips routing to axhub — not the repo or some hardcoded `Axhub`.
const IMPLICIT_DEPLOY_PROMPT: &str = "배포해";

/// Build a non-marker git repo in a fresh tempdir: a `.git` directory (so the
/// walk-up terminates deterministically at the repo root and cannot escape to
/// ancestors) and deliberately NO `axhub.yaml`. Mirrors the AC 1 / AC 9 fixture.
fn non_marker_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    tmp
}

// --- step 1: precondition — the repo is genuinely non-marker --------------------

/// A `.git` repo without `axhub.yaml` resolves to a confirmed `Absent` (not the
/// `Unknown` error fallback). Establishes the non-marker half independently so the
/// routing assertions rest on a verified input rather than an assumption.
#[test]
fn non_marker_repo_walk_up_is_absent() {
    let repo = non_marker_repo();
    let nested = repo.path().join("apps").join("api");
    std::fs::create_dir_all(&nested).expect("mkdir nested");
    assert_eq!(
        find_marker_from(&nested),
        MarkerStatus::Absent,
        "a .git repo without axhub.yaml must resolve to Absent, not Unknown"
    );
}

// --- step 2: the explicit keyword is detected (and is not a substring fluke) ----

/// Both phrasings name axhub explicitly and name no foreign target — so they
/// take the explicit-keyword arm (rule `b`), never the marker-driven arm. The
/// `foreign_keyword_present == false` half matters: it rules out rule `a`
/// (axhub + foreign → `Ask`) and rule `c` (foreign → `Yield`), pinning the route
/// to rule `b` for the right reason.
#[test]
fn explicit_keyword_prompts_are_axhub_keyword_only() {
    for prompt in [EXPLICIT_KEYWORD_PROMPT, EXPLICIT_KEYWORD_PROMPT_ALT] {
        assert!(
            axhub_keyword_present(prompt),
            "\"{prompt}\" must be read as an explicit axhub keyword"
        );
        assert!(
            !foreign_keyword_present(prompt),
            "\"{prompt}\" must not be read as naming a foreign target"
        );
    }
}

/// Non-vacuity guard for the detector: a near-miss substring must NOT fire the
/// keyword. If `axhub_keyword_present` were a naive `contains("axhub")`, this
/// would wrongly route `"axhubble"` to axhub and the routing assertions below
/// would be a substring tautology. Whole-word bounding is what makes the keyword
/// signal meaningful.
#[test]
fn near_miss_substring_does_not_trigger_axhub_keyword() {
    assert!(
        !axhub_keyword_present("axhubble deploy"),
        "\"axhubble\" must not satisfy the whole-word axhub keyword detector"
    );
}

// --- step 3: explicit keyword routes to Axhub, marker-independent, authed=false -

/// THE AC 2 contract. The explicit `"axhub에 배포해"` keyword routes to
/// [`RoutingDecision::Axhub`] in a non-marker repo across BOTH auth states. The
/// load-bearing row is `authed = false`: in a zero-footprint non-marker repo no
/// token-file exists (AC 7), yet the keyword must still route to axhub so the
/// deploy path — and its call-time lazy auth (proven by AC 9) — is reached.
#[test]
fn explicit_keyword_routes_to_axhub_in_non_marker_repo() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");

    for authed in [false, true] {
        let decision = decide(
            EXPLICIT_KEYWORD_PROMPT,
            marker,
            authed,
            /* explicit_invocation = */ false,
        );
        assert_eq!(
            decision,
            RoutingDecision::Axhub,
            "\"axhub에 배포해\" in a non-marker repo must route to axhub (authed={authed})"
        );
        // The wire form the routing-audit jsonl records for this row.
        assert_eq!(decision.as_str(), "axhub");
    }
}

/// Marker-independence increment over AC 9 (which exercised only `Absent` for the
/// keyword row): rule `b` ignores marker entirely, so the explicit keyword routes
/// to `Axhub` for `Present`, `Absent`, AND the `Unknown` error fallback — even at
/// `authed = false`, where the bare-NL `Unknown` arm would instead `Ignore`. This
/// is what "function regardless of marker presence" means concretely: a transient
/// marker walk-up error never suppresses an explicit axhub keyword.
#[test]
fn explicit_keyword_is_marker_independent_including_unknown() {
    for marker in [
        MarkerStatus::Present,
        MarkerStatus::Absent,
        MarkerStatus::Unknown,
    ] {
        for authed in [false, true] {
            assert_eq!(
                decide(EXPLICIT_KEYWORD_PROMPT_ALT, marker, authed, false),
                RoutingDecision::Axhub,
                "explicit axhub keyword must route to axhub at marker={marker:?} authed={authed}"
            );
        }
    }
}

// --- step 4: discriminator — the keyword, not the repo, is what routes to axhub -

/// Non-vacuity: in the SAME non-marker repo and the SAME `authed = false` state,
/// the bare-NL prompt ("배포해", no keyword) must `Ignore` (rule `e`) while the
/// explicit keyword routes to `Axhub` (rule `b`). The only difference between the
/// two `decide` calls is the prompt, so this proves the `"axhub"` keyword is what
/// flips routing — not the repo, the auth state, or a hardcoded outcome. Without
/// this contrast, `explicit_keyword_routes_to_axhub_in_non_marker_repo` would
/// still pass even if `decide` always returned `Axhub`.
#[test]
fn bare_nl_ignores_where_explicit_keyword_routes_to_axhub() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");

    let implicit = decide(IMPLICIT_DEPLOY_PROMPT, marker, /* authed */ false, false);
    let explicit = decide(EXPLICIT_KEYWORD_PROMPT, marker, /* authed */ false, false);

    assert_eq!(
        implicit,
        RoutingDecision::Ignore,
        "bare NL in a non-marker repo must Ignore (the keyword-absent baseline)"
    );
    assert_eq!(
        explicit,
        RoutingDecision::Axhub,
        "the explicit axhub keyword must flip the SAME repo+auth to Axhub"
    );
    assert_ne!(
        implicit, explicit,
        "the explicit keyword must change the routing decision — else the route is vacuous"
    );
}

//! spec 006 (AC 17) — a marker walk-up that **errors** + an **authed** user must
//! fail **open** to axhub routing.
//!
//! ## The scenario, concretely
//!
//! The project marker is `axhub.yaml`, located by walking up from the cwd to the
//! git root (spec 006 §23-30). That walk-up is local filesystem I/O, so it can
//! *fail* — a permission race, a stat error, a path component that is not a
//! directory. The routing model represents this third outcome as a distinct
//! tri-state [`MarkerStatus::Unknown`], deliberately **not** collapsed into
//! `Absent`, so a transient error is never mistaken for "confirmed no marker".
//!
//! On that error path the shared decision falls open **auth-conditionally**
//! (spec 006 §99, the `fail-open-safety` principle): an authed user keeps their
//! `"배포해" → axhub` behavior straight through the transient error, while an
//! unauthed user stays zero-footprint (`Ignore` / pass-through — the sibling AC).
//! This file pins the **authed** half: `Unknown + authed → Axhub`.
//!
//! ## What this file pins — and the discriminators that make it non-vacuous
//!
//!  1. **Reachability** — a real walk-up over a path whose ancestor is a *file*
//!     (so a child stat errors with `NotADirectory`) resolves to
//!     [`MarkerStatus::Unknown`], **not** `Absent`. This proves the `Unknown`
//!     branch is reachable from genuine I/O, not just hand-fed to the pure
//!     function. Unix-guarded: the `ENOTDIR` mapping is a POSIX guarantee that
//!     `root` cannot bypass (unlike a permission-based `EACCES` induction).
//!  2. **Endpoint** — `decide(bare-NL, Unknown, authed=true, explicit=false)` →
//!     [`RoutingDecision::Axhub`]. The fail-open routes the authed user to axhub.
//!  3. **Discriminator A (the auth gate is load-bearing)** — the *same* bare-NL
//!     prompt with `authed=false` → [`RoutingDecision::Ignore`]. Without this,
//!     the endpoint could pass for a reason other than the auth-conditional gate.
//!     This is the sibling AC's "unauthed → pass-through" outcome, used here only
//!     as the discriminator that proves auth is what flips the result.
//!  4. **Discriminator B (the *error* path is what drives it, not a marker)** —
//!     the same prompt with `authed=true` but marker `Absent` → `Ignore`. A
//!     confirmed-absent marker does **not** fail open even for an authed user
//!     (rule e); only the `Unknown` *error* does (rule err). This isolates the
//!     err branch from rules d/e.
//!
//! The prompt is held constant (`"배포해"` — no `"axhub"`, no foreign target, no
//! leading slash) across every row so the only thing that varies is
//! `(marker, authed)`. That is what makes the `Axhub` outcome attributable to the
//! err branch rather than to an explicit keyword/slash rule firing on the text.
//!
//! **Deferred — not tested here:** the live hook binary (`prompt-route`) computes
//! the marker via `find_marker()`, which is anchored at the *process* cwd; a live
//! process cannot be made to walk up through an `ENOTDIR` path, so the err branch
//! is only reachable at the `find_marker_from(path)` / `decide(...)` altitude. The
//! hook/preflight action mapping for `Axhub` is owned by `hook-integration-complete`
//! / `preflight-integration-complete`; the session-eager-gate auth-conditional
//! fallback on `Unknown` is owned by `session-start-gated` (AC 7). This file pins
//! the routing decision only.

use axhub_helpers::routing::{
    axhub_keyword_present, decide, decide_from_flags, find_marker_from, foreign_keyword_present,
    is_slash_invocation, MarkerStatus, RoutingDecision,
};

/// The bare natural-language deploy prompt. Held constant across every row below:
/// no `"axhub"` keyword (rule b), no foreign target (rule c), no leading slash
/// (rule 0) — so routing falls through to the marker arm and the `(marker, authed)`
/// pair is the *only* thing that can change the outcome.
const BARE_NL_PROMPT: &str = "배포해";

/// Guard: the constant prompt really is bare NL. If this ever regresses (someone
/// edits the prompt to contain "axhub" / a foreign keyword / a slash), every
/// assertion below would pass for the WRONG reason — an explicit rule firing, not
/// the err branch. Pin the precondition so that can't happen silently.
#[test]
fn prompt_is_genuinely_bare_nl() {
    assert!(
        !axhub_keyword_present(BARE_NL_PROMPT),
        "prompt must not contain the axhub keyword (would trigger rule b)"
    );
    assert!(
        !foreign_keyword_present(BARE_NL_PROMPT),
        "prompt must not name a foreign target (would trigger rule c)"
    );
    assert!(
        !is_slash_invocation(BARE_NL_PROMPT),
        "prompt must not be a slash invocation (would trigger rule 0)"
    );
}

// --- step 1: reachability — a real walk-up error resolves to Unknown ------------

/// A walk-up whose path traverses *through a regular file* makes the child stat
/// fail with `NotADirectory`, which `find_marker_from` surfaces as
/// [`MarkerStatus::Unknown`] (not `Absent`). This proves the err branch is
/// reachable from genuine filesystem I/O.
///
/// Unix-only: `ENOTDIR` is a POSIX guarantee independent of privilege (a `root`
/// CI runner still cannot descend into a file), so this is robust where a
/// permission-based induction (`chmod 000`) would be bypassed by `root`.
#[cfg(unix)]
#[test]
fn walk_up_through_a_file_is_unknown_not_absent() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let a_file = tmp.path().join("not-a-dir");
    std::fs::write(&a_file, b"i am a regular file\n").expect("write regular file");

    // `start` descends *through* the regular file, so resolving any child of it
    // errors with NotADirectory rather than NotFound.
    let start = a_file.join("sub").join("deep");
    assert_eq!(
        find_marker_from(&start),
        MarkerStatus::Unknown,
        "a walk-up that stat-errors must be Unknown, never Absent — Absent would \
         wrongly collapse a transient error into 'confirmed no marker'"
    );
}

// --- step 2: endpoint — Unknown + authed → Axhub (the fail-open) ----------------

/// The headline: on the marker-error path, an authed user's bare-NL deploy still
/// routes to axhub. Asserted through both the public `decide` wrapper (which
/// derives the keyword flags from the prompt) and the pure `decide_from_flags`
/// core, so neither input-construction nor the priority chain can drift the result.
#[test]
fn unknown_marker_authed_falls_open_to_axhub() {
    let via_wrapper = decide(
        BARE_NL_PROMPT,
        MarkerStatus::Unknown,
        /* authed = */ true,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        via_wrapper,
        RoutingDecision::Axhub,
        "marker walk-up error + authed user must fail open to axhub routing"
    );

    let via_core = decide_from_flags(
        /* axhub_keyword = */ false,
        /* foreign_keyword = */ false,
        MarkerStatus::Unknown,
        /* authed = */ true,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        via_core, via_wrapper,
        "the pure core must agree with the prompt-derived wrapper (no drift)"
    );
}

// --- step 3: discriminator A — the auth gate is what flips the result -----------

/// Same `Unknown` error path, same bare-NL prompt, but `authed=false` → `Ignore`.
/// This is the sibling AC's "unauthed → pass-through" outcome; here it serves as
/// the discriminator proving the endpoint's `Axhub` is produced by the
/// *auth-conditional* gate (rule err), not by the `Unknown` status alone.
#[test]
fn unknown_marker_unauthed_stays_pass_through() {
    let decision = decide(
        BARE_NL_PROMPT,
        MarkerStatus::Unknown,
        /* authed = */ false,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        decision,
        RoutingDecision::Ignore,
        "marker error + UNAUTHED must stay zero-footprint (pass-through), not axhub"
    );
}

// --- step 4: discriminator B — only the *error* fails open, not Absent ----------

/// Same authed user, same bare-NL prompt, but a *confirmed* `Absent` marker →
/// `Ignore` (rule e). A genuinely marker-less repo does NOT fail open even for an
/// authed user — that would defeat zero-footprint. Only the `Unknown` *error*
/// path (rule err) does. This isolates the err branch from rule e and proves the
/// endpoint is driven by the error, not by being authed.
#[test]
fn absent_marker_authed_does_not_fall_open() {
    let decision = decide(
        BARE_NL_PROMPT,
        MarkerStatus::Absent,
        /* authed = */ true,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        decision,
        RoutingDecision::Ignore,
        "a confirmed-absent marker must stay pass-through even for an authed user — \
         only the Unknown error path fails open"
    );
}

// --- headline: the full auth-conditional err-branch truth table, one place ------

/// All three err/absent rows co-located so a break in EITHER half of the
/// auth-conditional fail-open — or a regression that makes `Absent` fail open —
/// goes red here. The prompt and explicit flag are constant; only `(marker, authed)`
/// varies. This IS "marker walk-up error + authed user → fail-open to axhub routing",
/// shown to be both correct (authed→Axhub) and bounded (unauthed→Ignore,
/// Absent→Ignore).
#[test]
fn err_branch_auth_conditional_truth_table() {
    let cases = [
        // (marker, authed, expected, why)
        (
            MarkerStatus::Unknown,
            true,
            RoutingDecision::Axhub,
            "err + authed → fail open to axhub",
        ),
        (
            MarkerStatus::Unknown,
            false,
            RoutingDecision::Ignore,
            "err + unauthed → pass-through",
        ),
        (
            MarkerStatus::Absent,
            true,
            RoutingDecision::Ignore,
            "absent + authed → pass-through (not the err path)",
        ),
    ];
    for (marker, authed, expected, why) in cases {
        let got = decide(BARE_NL_PROMPT, marker, authed, false);
        assert_eq!(
            got, expected,
            "{why} (marker={marker:?} authed={authed})"
        );
    }
}

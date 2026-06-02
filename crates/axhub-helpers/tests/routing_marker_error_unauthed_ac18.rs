//! spec 006 (AC 18) — a marker walk-up that **errors** + an **unauthed** user must
//! fail **open to pass-through** (zero-footprint `Ignore`, never axhub routing).
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
//! (spec 006 §99, the `fail-open-safety` principle): an unauthed user stays
//! zero-footprint (`Ignore` / pass-through — no token-init, no warmup, no routing
//! nudge), while an authed user keeps their `"배포해" → axhub` behavior (the
//! sibling AC 17). This file pins the **unauthed** half: `Unknown + unauthed →
//! Ignore`.
//!
//! ## Why this AC's headline is *vacuous on its own* — and what rescues it
//!
//! For an **unauthed** user, `Unknown` (rule err) and `Absent` (rule e) both
//! resolve to `Ignore` — they are outcome-identical (`routing.rs` lines 139 & 147).
//! So the bare assertion `decide(bare-NL, Unknown, unauthed) == Ignore` would still
//! pass under the *exact bug* `MarkerStatus::Unknown` exists to prevent: collapsing
//! a walk-up error into `Absent`. The headline outcome therefore proves almost
//! nothing by itself. The test's discriminating power rests entirely on two things,
//! made central here rather than incidental:
//!
//!  1. **Reachability (the load-bearing guard)** — a real walk-up *through a
//!     regular file* (so a child stat errors with `NotADirectory`) resolves to
//!     [`MarkerStatus::Unknown`], **not** `Absent`. This proves the err branch is
//!     reachable from genuine I/O and is held distinct from `Absent`. Unix-guarded:
//!     `ENOTDIR` is a POSIX guarantee that `root` cannot bypass (unlike a
//!     permission-based `EACCES` induction).
//!  2. **The authed-contrast rows** — the *same* `Unknown` error path with
//!     `authed=true` → [`RoutingDecision::Axhub`], while `Absent + authed` →
//!     `Ignore`. Together these prove `Unknown` is routed *distinctly* from
//!     `Absent` (only the error fails open for an authed user), so the unauthed
//!     `Ignore` is the auth-conditional fail-open — not a blanket `Unknown→Ignore`
//!     nor an `Unknown`-collapsed-to-`Absent` bug.
//!
//! ## Value-adds over the sibling AC 17 file
//!
//!  - **Audit linkage** — the pass-through decision is what gets *recorded* as
//!    `"ignore"` for the routing-audit jsonl / routing-stats skill: this file pins
//!    `AuditDecision::from_routing(Ignore, explicit=false) → "ignore"`.
//!  - **Pass-through is bounded** — the same `Unknown` error path + unauthed but
//!    with an explicit slash invocation → `Axhub`. Pass-through does not swallow
//!    explicit invocations; an unauthed user's `/deploy` still routes to axhub
//!    through the transient error (the routing half of explicit-always-works).
//!
//! The prompt is held constant (`"배포해"` — no `"axhub"`, no foreign target, no
//! leading slash) so the only thing varying across the bare-NL rows is
//! `(marker, authed)`; the `Ignore` outcome is thus attributable to the err/absent
//! marker arm, not to an explicit keyword/slash rule firing on the text.
//!
//! **Deferred — not tested here:** the live hook binary (`prompt-route`) computes
//! the marker via `find_marker()`, anchored at the *process* cwd; a live process
//! cannot be made to walk up through an `ENOTDIR` path (`Command::current_dir`
//! requires a real directory), so the err branch is reachable only at the
//! `find_marker_from(path)` / `decide(...)` altitude. The hook/preflight action
//! mapping for `Ignore` (silent + authed-only grace) is owned by
//! `hook-integration-complete` / `preflight-integration-complete`; the
//! session-eager-gate auth-conditional fallback on `Unknown` is owned by
//! `session-start-gated` (AC 7). This file pins the routing decision only.

use axhub_helpers::audit::AuditDecision;
use axhub_helpers::routing::{
    axhub_keyword_present, decide, decide_from_flags, find_marker_from, foreign_keyword_present,
    is_slash_invocation, MarkerStatus, RoutingDecision,
};

/// The bare natural-language deploy prompt. Held constant across every bare-NL row
/// below: no `"axhub"` keyword (rule b), no foreign target (rule c), no leading
/// slash (rule 0) — so routing falls through to the marker arm and the
/// `(marker, authed)` pair is the *only* thing that can change the outcome.
const BARE_NL_PROMPT: &str = "배포해";

/// Guard: the constant prompt really is bare NL. If this ever regresses (someone
/// edits the prompt to contain "axhub" / a foreign keyword / a slash), every
/// assertion below would pass for the WRONG reason — an explicit rule firing, not
/// the err/absent marker arm. Pin the precondition so that can't happen silently.
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

// --- step 1: reachability — the load-bearing guard ------------------------------

/// A walk-up whose path traverses *through a regular file* makes the child stat
/// fail with `NotADirectory`, which `find_marker_from` surfaces as
/// [`MarkerStatus::Unknown`] (not `Absent`). This is THE load-bearing guard for
/// this AC: because `Unknown` and `Absent` are outcome-identical for an unauthed
/// user, the only thing proving the err branch is genuinely exercised (and not
/// silently collapsed into `Absent`) is that real filesystem I/O resolves the
/// error to `Unknown`.
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

/// End-to-end on the *unauthed* path: feed the real walk-up error straight into
/// `decide_from_flags` (bare-NL flags) and confirm the genuine I/O error routes to
/// `Ignore`. This chains step 1 (error → `Unknown`) into the headline outcome
/// without hand-feeding `Unknown`, so the pass-through is shown to be driven by an
/// actual filesystem error, not just a synthetic enum value.
#[cfg(unix)]
#[test]
fn real_walk_up_error_unauthed_routes_to_ignore() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let a_file = tmp.path().join("not-a-dir");
    std::fs::write(&a_file, b"regular file\n").expect("write regular file");
    let start = a_file.join("sub").join("deep");

    let marker = find_marker_from(&start);
    assert_eq!(marker, MarkerStatus::Unknown, "precondition: error → Unknown");

    let decision = decide_from_flags(
        /* axhub_keyword = */ false,
        /* foreign_keyword = */ false,
        marker,
        /* authed = */ false,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        decision,
        RoutingDecision::Ignore,
        "a genuine marker walk-up error + unauthed user must fail open to \
         pass-through (Ignore), never axhub routing"
    );
}

// --- step 2: headline — Unknown + unauthed → Ignore (the pass-through) ----------

/// The headline (made non-vacuous by steps 1 and 3): on the marker-error path, an
/// unauthed user's bare-NL deploy stays zero-footprint pass-through. Asserted
/// through both the public `decide` wrapper (which derives the keyword flags from
/// the prompt) and the pure `decide_from_flags` core, so neither input-construction
/// nor the priority chain can drift the result.
#[test]
fn unknown_marker_unauthed_falls_open_to_pass_through() {
    let via_wrapper = decide(
        BARE_NL_PROMPT,
        MarkerStatus::Unknown,
        /* authed = */ false,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        via_wrapper,
        RoutingDecision::Ignore,
        "marker walk-up error + unauthed user must fail open to pass-through (Ignore)"
    );

    let via_core = decide_from_flags(
        /* axhub_keyword = */ false,
        /* foreign_keyword = */ false,
        MarkerStatus::Unknown,
        /* authed = */ false,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        via_core, via_wrapper,
        "the pure core must agree with the prompt-derived wrapper (no drift)"
    );
}

// --- step 3: discriminators — Unknown is routed distinctly from Absent ----------

/// Discriminator A — the *same* `Unknown` error path with `authed=true` → `Axhub`
/// (the sibling AC 17). If this and the headline both hold, then `authed` is what
/// flips the err-branch outcome: the unauthed `Ignore` is the auth-conditional
/// fail-open, not a blanket `Unknown→Ignore`.
#[test]
fn unknown_marker_authed_falls_open_to_axhub() {
    let decision = decide(
        BARE_NL_PROMPT,
        MarkerStatus::Unknown,
        /* authed = */ true,
        /* explicit_invocation = */ false,
    );
    assert_eq!(
        decision,
        RoutingDecision::Axhub,
        "the SAME error path with an authed user must fail open to axhub — proving \
         the unauthed Ignore is auth-conditional, not blanket"
    );
}

/// Discriminator B — a *confirmed* `Absent` marker with `authed=true` → `Ignore`
/// (rule e). An authed user does NOT fail open on a genuinely marker-less repo;
/// only the `Unknown` *error* does (rule err). Paired with discriminator A this
/// proves `Unknown` is treated *distinctly* from `Absent` — the bug
/// `MarkerStatus::Unknown` exists to prevent (collapsing error into Absent) would
/// make discriminator A go red here.
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

// --- step 4: pass-through is bounded — explicit invocations still work ----------

/// The unauthed pass-through must NOT swallow explicit invocations. On the same
/// `Unknown` error path, an unauthed user's explicit slash invocation (rule 0) →
/// `Axhub`. This is the routing half of explicit-always-works / lazy-auth: a
/// transient marker error never downgrades a deliberate `/deploy` to pass-through.
#[test]
fn explicit_invocation_unauthed_survives_marker_error() {
    // Via the model-passed slash signal.
    assert_eq!(
        decide(
            BARE_NL_PROMPT,
            MarkerStatus::Unknown,
            /* authed = */ false,
            /* explicit_invocation = */ true,
        ),
        RoutingDecision::Axhub,
        "explicit invocation must override the unauthed marker-error pass-through"
    );
    // And via a slash left in the utterance text. `decide` does NOT itself parse
    // the prompt for a slash — the consumers (hook / route-decision) derive
    // `explicit = flag || is_slash_invocation(utterance)` at the call site, so the
    // detector is composed here exactly as they compose it.
    let slash_prompt = "/deploy";
    assert_eq!(
        decide(
            slash_prompt,
            MarkerStatus::Unknown,
            /* authed = */ false,
            /* explicit_invocation = */ is_slash_invocation(slash_prompt),
        ),
        RoutingDecision::Axhub,
        "a slash in the utterance is detected as explicit → axhub even unauthed on error"
    );
}

// --- step 5: audit linkage — the pass-through is recorded as "ignore" -----------

/// The pass-through decision must surface in the routing-audit trail as `"ignore"`
/// (spec 006 §80/§94) so routing-stats can report the non-axhub pass-through rate.
/// `explicit_invocation=false` because the headline scenario is bare NL; an
/// explicit path would label `"explicit"` (and route `Axhub`, per step 4).
#[test]
fn pass_through_decision_audits_as_ignore() {
    let label = AuditDecision::from_routing(RoutingDecision::Ignore, /* explicit = */ false);
    assert_eq!(
        label,
        AuditDecision::Ignore,
        "a pass-through (Ignore) decision must map to the Ignore audit label"
    );
    assert_eq!(
        label.as_str(),
        "ignore",
        "the audit wire string for pass-through must be \"ignore\""
    );
}

// --- headline: the unauthed err-branch truth table, one place -------------------

/// All the rows that make the unauthed pass-through both correct and bounded,
/// co-located so a break in any leg goes red here. Prompt held constant; only
/// `(marker, authed, explicit)` varies. This IS "marker walk-up error + unauthed
/// user → fail-open to pass-through", shown to be:
///   - the stated outcome              (Unknown + unauthed + bare-NL → Ignore),
///   - auth-conditional, not blanket   (Unknown + authed   + bare-NL → Axhub),
///   - distinct from Absent            (Absent  + authed   + bare-NL → Ignore),
///   - and bounded (does not swallow explicit) (Unknown + unauthed + explicit → Axhub).
#[test]
fn unauthed_err_branch_truth_table() {
    let cases = [
        // (marker, authed, explicit, expected, why)
        (
            MarkerStatus::Unknown,
            false,
            false,
            RoutingDecision::Ignore,
            "err + unauthed (bare NL) → fail open to pass-through",
        ),
        (
            MarkerStatus::Unknown,
            true,
            false,
            RoutingDecision::Axhub,
            "err + authed (bare NL) → fail open to axhub (auth gate is load-bearing)",
        ),
        (
            MarkerStatus::Absent,
            true,
            false,
            RoutingDecision::Ignore,
            "absent + authed (bare NL) → pass-through (only the error fails open)",
        ),
        (
            MarkerStatus::Unknown,
            false,
            true,
            RoutingDecision::Axhub,
            "err + unauthed + explicit → axhub (pass-through does not swallow /deploy)",
        ),
    ];
    for (marker, authed, explicit, expected, why) in cases {
        let got = decide(BARE_NL_PROMPT, marker, authed, explicit);
        assert_eq!(
            got, expected,
            "{why} (marker={marker:?} authed={authed} explicit={explicit})"
        );
    }
}

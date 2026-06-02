//! Shared routing-decision logic — the **single source of truth** consumed by
//! both axhub trigger paths:
//!
//! 1. the prompt-route hook (`UserPromptSubmit`, runs first), and
//! 2. the deploy SKILL preflight (Step 0, runs when the skill is selected).
//!
//! Keeping the decision in one pure function makes logic drift between the two
//! paths *structurally impossible*: if both layers call [`decide`], they cannot
//! disagree for the same inputs. This is the mechanism behind the
//! "composition-consistency" guarantee (spec 006 §49-59, "공유 routing-decision
//! 함수"). The classic drift this prevents: the hook yields to `"vercel"` while
//! the preflight sees only `marker_present` and routes to axhub anyway — which
//! reproduces the original "Vercel 쓰고 싶은데 axhub 로 라우팅" complaint on every
//! marker-present repo (i.e. every developer's own repo). See [`decide_from_flags`].
//!
//! ## Design
//!
//! - [`decide_from_flags`] is the **pure ordered priority chain** over already
//!   computed booleans — exhaustively matrix-testable, no I/O. The *order* of
//!   the `if`/`else` arms is the load-bearing logic, not the individual rule
//!   outputs.
//! - [`decide`] is the thin public wrapper that derives the keyword flags from
//!   the raw prompt via the shared detectors ([`axhub_keyword_present`],
//!   [`foreign_keyword_present`]) so both layers derive inputs identically too
//!   (input-construction is itself a drift surface).
//! - Marker presence is modeled as a tri-state [`MarkerStatus`] so a walk-up
//!   that *errors* (fs permission / race) falls open auth-conditionally
//!   (spec 006 §99) instead of silently collapsing the error into "absent".

use std::path::Path;

/// Foreign deploy-target keywords. Hardcoded per spec 006 §45-47 — a
/// slow-changing set, intentionally not externalized. Presence of any of these
/// (with no explicit `"axhub"`) means the user named another target, so axhub
/// yields ("named target wins").
pub const FOREIGN_TARGET_KEYWORDS: &[&str] =
    &["vercel", "netlify", "cloudflare", "fly", "render", "railway"];

/// The literal keyword that marks an explicit axhub intent (marker-independent).
pub const AXHUB_KEYWORD: &str = "axhub";

/// Defensive cap on marker walk-up depth so a pathological filesystem can never
/// spin the hot path. 64 levels is far deeper than any real project tree.
const MAX_WALK_UP_DEPTH: usize = 64;

/// Outcome of the shared routing decision.
///
/// Serialized lowercase (`"axhub"` / `"yield"` / `"ignore"` / `"ask"`) for the
/// routing-audit jsonl + routing-stats skill (spec 006 §94). Intentionally
/// **not** `#[non_exhaustive]`: consumers must handle all four arms, and adding
/// a variant later *should* force them to update their action mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingDecision {
    /// Proceed with axhub routing. hook → neutral (allow skill-selection);
    /// preflight → proceed with deploy.
    Axhub,
    /// Yield to the normal flow (user named a foreign target). hook → silent;
    /// preflight → yield to general flow.
    Yield,
    /// Not an axhub intent (no marker, bare NL). hook → silent (+ once-per-project
    /// grace when authed); preflight → disambiguation.
    Ignore,
    /// Ambiguous (axhub + foreign both named). hook → neutral (cannot run tools;
    /// disambiguation is owned by the preflight); preflight → disambiguation.
    Ask,
}

impl RoutingDecision {
    /// Lowercase wire form, matching the spec's decision literals and the serde
    /// representation. Handy for audit logging without a serde round-trip.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RoutingDecision::Axhub => "axhub",
            RoutingDecision::Yield => "yield",
            RoutingDecision::Ignore => "ignore",
            RoutingDecision::Ask => "ask",
        }
    }
}

/// Tri-state result of the marker walk-up.
///
/// `Unknown` distinguishes a genuine filesystem error (permission / race)
/// from a confirmed `Absent`, so [`decide_from_flags`] can apply the
/// auth-conditional fail-open (spec 006 §99) only on real errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerStatus {
    /// `axhub.yaml` found via cwd→git-root walk-up.
    Present,
    /// Walk-up completed (git root or fs root reached) with no `axhub.yaml`.
    Absent,
    /// Walk-up could not complete because a filesystem stat errored.
    Unknown,
}

/// The pure, ordered routing-decision priority chain (spec 006 §32-43 rules
/// `0`..`e` + the `err` fallback). This function does **no I/O** — feed it
/// already-computed flags. The arm order encodes precedence and is the part
/// AC-16 exists to lock; do not reorder without updating the matrix tests.
///
/// Priority (first match wins):
/// - **0** `explicit_invocation` (slash `/deploy`, `/axhub:deploy`) → `Axhub`
///   (strongest explicit signal; beats every keyword conflict).
/// - **a** `axhub_keyword` AND `foreign_keyword` → `Ask` (disambiguate).
/// - **b** `axhub_keyword` → `Axhub` (explicit, marker-independent).
/// - **c** `foreign_keyword` → `Yield` (named target wins; beats marker).
/// - **d** bare NL + marker `Present` → `Axhub`.
/// - **e** bare NL + marker `Absent` → `Ignore` (grace is consumer-side).
/// - **err** bare NL + marker `Unknown` → `authed ? Axhub : Ignore`.
#[must_use]
pub fn decide_from_flags(
    axhub_keyword: bool,
    foreign_keyword: bool,
    marker: MarkerStatus,
    authed: bool,
    explicit_invocation: bool,
) -> RoutingDecision {
    // rule 0 — slash invocation: strongest explicit, above keyword conflict.
    if explicit_invocation {
        return RoutingDecision::Axhub;
    }
    // rule a — both axhub and a foreign target named: ambiguous, disambiguate.
    if axhub_keyword && foreign_keyword {
        return RoutingDecision::Ask;
    }
    // rule b — explicit "axhub" keyword (no foreign): marker-independent.
    if axhub_keyword {
        return RoutingDecision::Axhub;
    }
    // rule c — foreign target named (no axhub): yield. Beats marker → "named target wins".
    if foreign_keyword {
        return RoutingDecision::Yield;
    }
    // bare NL (no explicit keyword): the marker decides.
    match marker {
        MarkerStatus::Present => RoutingDecision::Axhub, // rule d
        MarkerStatus::Absent => RoutingDecision::Ignore, // rule e (grace handled by consumer)
        // err — marker walk-up errored: fall open auth-conditionally (spec §99).
        // Authed users keep their "배포해"→axhub behavior through transient fs
        // errors; unauthed users stay zero-footprint (pass-through / ignore).
        MarkerStatus::Unknown => {
            if authed {
                RoutingDecision::Axhub
            } else {
                RoutingDecision::Ignore
            }
        }
    }
}

/// Public entry point: derive keyword flags from the raw `prompt` (via the
/// shared detectors) and run [`decide_from_flags`]. Both the hook and the deploy
/// preflight call this so keyword derivation cannot drift between them either.
#[must_use]
pub fn decide(
    prompt: &str,
    marker: MarkerStatus,
    authed: bool,
    explicit_invocation: bool,
) -> RoutingDecision {
    decide_from_flags(
        axhub_keyword_present(prompt),
        foreign_keyword_present(prompt),
        marker,
        authed,
        explicit_invocation,
    )
}

/// True when `prompt` names axhub explicitly. Whole-word match (see
/// [`contains_word`]) so it fires on `"axhub 배포"` and `"axhub.yaml"` but the
/// match is bounded — no surprise substring hits.
#[must_use]
pub fn axhub_keyword_present(prompt: &str) -> bool {
    contains_word(&prompt.to_lowercase(), AXHUB_KEYWORD)
}

/// True when `prompt` names any [`FOREIGN_TARGET_KEYWORDS`] target. Whole-word
/// match so `"render the page"` / `"fly.io"` fire but `"rendered"` /
/// `"butterfly"` do not.
#[must_use]
pub fn foreign_keyword_present(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    FOREIGN_TARGET_KEYWORDS
        .iter()
        .any(|kw| contains_word(&lower, kw))
}

/// True when `prompt` is an explicit slash invocation of an axhub command
/// (`/deploy`, `/배포` — the Korean alias in `commands/배포.md`, `/axhub:…`). The
/// deploy preflight detects this from its own invocation context; the hook detects
/// it from the prompt text — both feed the result into [`decide`] as
/// `explicit_invocation`. `/배포` is included because it is a first-class slash
/// command (Korean-first plugin), and the command forwards only `$ARGUMENTS`, so
/// this detector is the fallback when the leading token survives in the text.
#[must_use]
pub fn is_slash_invocation(prompt: &str) -> bool {
    // Match the **exact** leading command token, not a bare prefix: `/deployment`
    // and `/배포해` are different tokens and must not be mistaken for the `/deploy`
    // / `/배포` commands (over-detection would route unrelated slash text to axhub
    // via priority rule 0). The `/axhub:` namespace stays a prefix — any
    // `/axhub:<cmd>` is an explicit axhub invocation.
    let first = prompt.trim_start().split_whitespace().next().unwrap_or("");
    first == "/deploy" || first == "/배포" || first.starts_with("/axhub:")
}

/// Whole-word substring search. `keyword` is assumed lowercase ASCII; `haystack`
/// must already be lowercased by the caller. A hit must be bounded on both sides
/// by a non-ASCII-alphanumeric byte (or a string edge), so foreign keywords like
/// `"fly"`/`"render"` never fire inside `"butterfly"`/`"rendered"`. Non-ASCII
/// bytes (e.g. Korean) count as boundaries, so `"vercel로"` still matches.
fn contains_word(haystack: &str, keyword: &str) -> bool {
    let kw = keyword.as_bytes();
    let hay = haystack.as_bytes();
    if kw.is_empty() || hay.len() < kw.len() {
        return false;
    }
    let mut i = 0;
    while i + kw.len() <= hay.len() {
        if &hay[i..i + kw.len()] == kw {
            let before_ok = i == 0 || !hay[i - 1].is_ascii_alphanumeric();
            let after = i + kw.len();
            let after_ok = after == hay.len() || !hay[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Walk up from `start` looking for `axhub.yaml`, stopping at the first `.git`
/// directory (the git root) and falling back to the filesystem root for
/// non-git trees (spec 006 §23-30). Local fs checks only — never any network.
///
/// Returns [`MarkerStatus::Unknown`] on a filesystem error so the caller can
/// fall open auth-conditionally rather than mistaking an error for `Absent`.
#[must_use]
pub fn find_marker_from(start: &Path) -> MarkerStatus {
    for dir in start.ancestors().take(MAX_WALK_UP_DEPTH) {
        match dir.join("axhub.yaml").try_exists() {
            Ok(true) => return MarkerStatus::Present,
            Ok(false) => {}
            Err(_) => return MarkerStatus::Unknown,
        }
        // Stop the walk-up at the git root: check it AFTER axhub.yaml so a
        // marker living at the git root is still found.
        match dir.join(".git").try_exists() {
            Ok(true) => return MarkerStatus::Absent,
            Ok(false) => {}
            Err(_) => return MarkerStatus::Unknown,
        }
    }
    MarkerStatus::Absent
}

/// [`find_marker_from`] anchored at the current working directory. Returns
/// [`MarkerStatus::Unknown`] if the cwd itself cannot be read.
#[must_use]
pub fn find_marker() -> MarkerStatus {
    match std::env::current_dir() {
        Ok(cwd) => find_marker_from(&cwd),
        Err(_) => MarkerStatus::Unknown,
    }
}

/// Cheap "is the user axhub-authed?" probe: a `.exists()` stat on the helper
/// auth/delegation token-file (`~/.config/axhub-plugin/token`, spec 006 §102).
///
/// MUST stay a pure stat — never spawn `axhub auth status` and never trigger
/// token-init bootstrap, or we create an "auth-read → bootstrap → marker-gate"
/// cycle. Token presence is a *proxy* for authed (a CLI-authed user with no
/// helper token reads as not-authed → pass-through, an accepted under-detection
/// on the error path). NOTE: this is the auth token, distinct from
/// `consent::…` HMAC consent tokens — do not conflate.
#[must_use]
pub fn token_present() -> bool {
    crate::runtime_paths::token_file()
        .map(|path| path.try_exists().unwrap_or(false))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Independent reference implementation of the spec 006 priority table,
    /// written deliberately differently from [`decide_from_flags`] (explicit
    /// nested matches instead of an early-return chain). The exhaustive matrix
    /// test below asserts the production chain agrees with this reference for
    /// *every* input combination — that agreement is the no-drift lock.
    fn reference_decision(
        axhub: bool,
        foreign: bool,
        marker: MarkerStatus,
        authed: bool,
        explicit: bool,
    ) -> RoutingDecision {
        if explicit {
            RoutingDecision::Axhub
        } else if axhub && foreign {
            RoutingDecision::Ask
        } else if axhub {
            RoutingDecision::Axhub
        } else if foreign {
            RoutingDecision::Yield
        } else {
            match (marker, authed) {
                (MarkerStatus::Present, _) => RoutingDecision::Axhub,
                (MarkerStatus::Absent, _) => RoutingDecision::Ignore,
                (MarkerStatus::Unknown, true) => RoutingDecision::Axhub,
                (MarkerStatus::Unknown, false) => RoutingDecision::Ignore,
            }
        }
    }

    /// Exhaustive 2×2×3×2×2 = 48-combo matrix. Locks the full priority ordering
    /// — every collision (0 > all, a > b/c, c > d) is exercised because every
    /// combination is enumerated. If the production chain ever reorders, this
    /// diverges from the reference and fails.
    #[test]
    fn decide_from_flags_matches_reference_for_all_inputs() {
        let markers = [
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ];
        let mut count = 0;
        for &axhub in &[false, true] {
            for &foreign in &[false, true] {
                for &marker in &markers {
                    for &authed in &[false, true] {
                        for &explicit in &[false, true] {
                            let got = decide_from_flags(axhub, foreign, marker, authed, explicit);
                            let want = reference_decision(axhub, foreign, marker, authed, explicit);
                            assert_eq!(
                                got, want,
                                "drift at axhub={axhub} foreign={foreign} marker={marker:?} authed={authed} explicit={explicit}"
                            );
                            count += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(count, 48, "matrix must enumerate every input combination");
    }

    /// rule 0 — slash invocation beats every keyword/marker/auth combination.
    #[test]
    fn rule0_slash_invocation_always_wins() {
        let markers = [
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ];
        for &axhub in &[false, true] {
            for &foreign in &[false, true] {
                for &marker in &markers {
                    for &authed in &[false, true] {
                        assert_eq!(
                            decide_from_flags(axhub, foreign, marker, authed, true),
                            RoutingDecision::Axhub,
                            "slash must win at axhub={axhub} foreign={foreign} marker={marker:?} authed={authed}"
                        );
                    }
                }
            }
        }
    }

    /// THE drift case AC-16 exists to catch (spec §59): a foreign target named
    /// in a marker-present repo must `Yield` (rule c) — never route to `Axhub`
    /// off the marker (rule d). Asserted across marker/auth so the precedence,
    /// not an incidental input, is what holds.
    #[test]
    fn rulec_foreign_keyword_beats_marker_present() {
        for &marker in &[
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ] {
            for &authed in &[false, true] {
                assert_eq!(
                    decide_from_flags(false, true, marker, authed, false),
                    RoutingDecision::Yield,
                    "named-target-wins must hold at marker={marker:?} authed={authed}"
                );
            }
        }
    }

    /// rule a — axhub + foreign both named (no slash) → Ask, regardless of marker/auth.
    #[test]
    fn rulea_axhub_plus_foreign_asks() {
        for &marker in &[
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ] {
            for &authed in &[false, true] {
                assert_eq!(
                    decide_from_flags(true, true, marker, authed, false),
                    RoutingDecision::Ask
                );
            }
        }
    }

    /// rule b — "axhub" keyword alone is marker-independent → Axhub.
    #[test]
    fn ruleb_axhub_keyword_is_marker_independent() {
        for &marker in &[
            MarkerStatus::Present,
            MarkerStatus::Absent,
            MarkerStatus::Unknown,
        ] {
            assert_eq!(
                decide_from_flags(true, false, marker, false, false),
                RoutingDecision::Axhub
            );
        }
    }

    /// rules d / e — bare NL routed purely by marker presence.
    #[test]
    fn ruled_rulee_bare_nl_follows_marker() {
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Present, false, false),
            RoutingDecision::Axhub
        );
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Absent, true, false),
            RoutingDecision::Ignore
        );
    }

    /// err — bare NL + Unknown marker falls open auth-conditionally (spec §99).
    #[test]
    fn err_branch_is_auth_conditional() {
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Unknown, true, false),
            RoutingDecision::Axhub
        );
        assert_eq!(
            decide_from_flags(false, false, MarkerStatus::Unknown, false, false),
            RoutingDecision::Ignore
        );
    }

    /// The public [`decide`] wrapper must agree with [`decide_from_flags`] fed
    /// the shared detectors' output — i.e. prompt-derived keyword flags don't
    /// drift from the core chain. This is the input-construction half of no-drift.
    #[test]
    fn decide_wrapper_agrees_with_core_over_detectors() {
        let prompts = [
            "배포해",
            "axhub 으로 배포해",
            "vercel 로 배포해",
            "axhub 말고 vercel 로",
            "deploy this to render",
            "그냥 빌드만",
        ];
        for prompt in prompts {
            for &marker in &[
                MarkerStatus::Present,
                MarkerStatus::Absent,
                MarkerStatus::Unknown,
            ] {
                for &authed in &[false, true] {
                    for &explicit in &[false, true] {
                        let via_wrapper = decide(prompt, marker, authed, explicit);
                        let via_core = decide_from_flags(
                            axhub_keyword_present(prompt),
                            foreign_keyword_present(prompt),
                            marker,
                            authed,
                            explicit,
                        );
                        assert_eq!(
                            via_wrapper, via_core,
                            "wrapper/core drift on prompt {prompt:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn axhub_keyword_detection() {
        assert!(axhub_keyword_present("axhub 배포"));
        assert!(axhub_keyword_present("deploy with AXHUB now"));
        assert!(axhub_keyword_present("read axhub.yaml")); // bounded by '.'
        assert!(!axhub_keyword_present("배포해주세요"));
        assert!(!axhub_keyword_present("axhubble")); // no false substring hit
    }

    #[test]
    fn foreign_keyword_detection_is_whole_word() {
        assert!(foreign_keyword_present("vercel 로 올려줘"));
        assert!(foreign_keyword_present("push to render"));
        assert!(foreign_keyword_present("deploy on fly.io")); // bounded by '.'
        assert!(foreign_keyword_present("use Netlify"));
        // No false positives from substrings:
        assert!(!foreign_keyword_present("a butterfly landed")); // contains "fly"
        assert!(!foreign_keyword_present("it rendered fine")); // contains "render"
        assert!(!foreign_keyword_present("just a normal prompt"));
    }

    #[test]
    fn slash_invocation_detection() {
        assert!(is_slash_invocation("/deploy"));
        assert!(is_slash_invocation("  /deploy to prod"));
        assert!(is_slash_invocation("/axhub:deploy"));
        assert!(is_slash_invocation("/axhub:apps"));
        assert!(is_slash_invocation("/배포")); // Korean alias (commands/배포.md)
        assert!(is_slash_invocation("/배포 paydrop"));
        assert!(!is_slash_invocation("deploy"));
        assert!(!is_slash_invocation("please /deploy"));
        assert!(!is_slash_invocation("배포해")); // bare Korean NL is NOT a slash
        // Bounded to the exact command token: a prompt that merely *starts with*
        // "/deploy"/"/배포" but is a different token must NOT be treated as the
        // command (over-detection would route unrelated slash text to axhub via
        // rule 0).
        assert!(!is_slash_invocation("/deployment-plan 설명해줘"));
        assert!(!is_slash_invocation("/deploy-history"));
        assert!(!is_slash_invocation("/배포해")); // "/배포" + 해 = a different token
    }

    #[test]
    fn marker_found_walking_up_to_git_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // root/.git  + root/axhub.yaml ; cwd = root/a/b/c
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        std::fs::write(root.join("axhub.yaml"), "app: demo\n").expect("write marker");
        let nested = root.join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).expect("mkdir nested");
        assert_eq!(find_marker_from(&nested), MarkerStatus::Present);
    }

    #[test]
    fn marker_absent_stops_at_git_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // .git at root but NO axhub.yaml anywhere → Absent (walk stops at git root).
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        let nested = root.join("pkg").join("src");
        std::fs::create_dir_all(&nested).expect("mkdir nested");
        assert_eq!(find_marker_from(&nested), MarkerStatus::Absent);
    }

    #[test]
    fn marker_present_at_git_root_itself() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // Marker AND .git both at the git root: marker must still be found
        // (axhub.yaml is checked before the .git stop condition).
        std::fs::create_dir(root.join(".git")).expect("mkdir .git");
        std::fs::write(root.join("axhub.yaml"), "app: demo\n").expect("write marker");
        assert_eq!(find_marker_from(root), MarkerStatus::Present);
    }

    #[test]
    fn decision_wire_strings_match_spec() {
        assert_eq!(RoutingDecision::Axhub.as_str(), "axhub");
        assert_eq!(RoutingDecision::Yield.as_str(), "yield");
        assert_eq!(RoutingDecision::Ignore.as_str(), "ignore");
        assert_eq!(RoutingDecision::Ask.as_str(), "ask");
        // serde representation agrees with as_str().
        assert_eq!(
            serde_json::to_string(&RoutingDecision::Yield).expect("serialize"),
            "\"yield\""
        );
    }
}

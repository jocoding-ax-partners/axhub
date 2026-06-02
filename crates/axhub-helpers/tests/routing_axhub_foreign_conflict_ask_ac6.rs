//! spec 006 AC 6 — a prompt naming BOTH "axhub" AND a foreign deploy target in
//! the same utterance is the *ambiguous* case: the shared routing decision must
//! be `ask`, which the deploy SKILL preflight Step 0 turns into a single
//! AskUserQuestion disambiguation ("axhub 에 배포할까요, 아니면 다른 곳에 배포할까요?").
//!
//! ## Why this is its own file (not a dup of `routing_preflight_gate.rs`)
//!
//! `routing_preflight_gate.rs::axhub_plus_foreign_blocks_with_ask` already pins
//! the *consequence* (`.decision == "ask"`) for the vercel case. AC-6 is about
//! the **conflict condition** in the RoutingDecision ontology: the decision is
//! `ask` precisely because `axhub_keyword_present` ∧ `foreign_keyword_present`
//! are *both* true in one prompt. So this file:
//!   1. asserts the two ontology booleans (`axhub_keyword`, `foreign_keyword`)
//!      in the SAME `route-decision` output that yields `ask` — the gate test
//!      only extracts `.decision`, never the inputs;
//!   2. sweeps **all six** hardcoded foreign keywords (spec 006 §45-47), not just
//!      vercel, each in conflict with "axhub" → `ask`;
//!   3. proves the `ask` is invariant to marker/auth (ambiguity must never
//!      collapse into a marker- or auth-driven `axhub`/`ignore`);
//!   4. discriminates `ask` from the single-keyword decisions in one line each
//!      (axhub-only → `axhub`, foreign-only → `yield`) so "conflict" is what
//!      drives `ask`, not an incidental input.
//!
//! The subcommand under test is `route-decision` — the exact entry the deploy
//! SKILL preflight Step 0 calls before any auth/resolve, so asserting `ask` here
//! pins the input half of "disambiguation at preflight". The skill-layer half
//! (ask → AskUserQuestion, registry safe default "여기 말고 다른 곳") is pinned by
//! `tests/deploy-preflight-routing-gate.test.ts` + the SKILL Step 0 `ask` branch.
//!
//! Auth is driven via `XDG_CONFIG_HOME` (token_present() stats
//! `$XDG_CONFIG_HOME/axhub-plugin/token`) and the marker via the spawned cwd —
//! neither spawns the axhub CLI nor triggers token-init (spec §102).

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

/// The six hardcoded foreign deploy-target keywords (spec 006 §45-47), mirrored
/// here so a drift in `routing::FOREIGN_TARGET_KEYWORDS` surfaces as an AC-6
/// failure (this list is the AC-6 contract, not a re-export).
const FOREIGN_KEYWORDS: &[&str] = &[
    "vercel",
    "netlify",
    "cloudflare",
    "fly",
    "render",
    "railway",
];

/// Build a repo dir under a fresh tempdir. `.git` is always created so the marker
/// walk-up terminates at this root (cannot escape to a tempdir ancestor);
/// `axhub.yaml` is written only when `marker` is true.
fn repo(marker: bool) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    if marker {
        std::fs::write(tmp.path().join("axhub.yaml"), "app: demo\n").expect("write marker");
    }
    tmp
}

/// A config-home tempdir; when `authed`, it carries the token file that
/// `token_present()` stats. Returned so the dir outlives the spawn.
fn config_home(authed: bool) -> tempfile::TempDir {
    let cfg = tempfile::tempdir().expect("cfg tempdir");
    if authed {
        let dir = cfg.path().join("axhub-plugin");
        std::fs::create_dir_all(&dir).expect("mkdir axhub-plugin");
        std::fs::write(dir.join("token"), "axhub_pat_stub\n").expect("write token");
    }
    cfg
}

/// Run `route-decision` as the SKILL Step 0 does and return the full stdout JSON.
/// Asserts exit 0 (fail-open contract — the gate must never break the session).
fn route(cwd: &Path, config: &Path, utterance: &str, explicit: bool) -> String {
    let mut cmd = Command::new(BIN);
    cmd.current_dir(cwd)
        .env("XDG_CONFIG_HOME", config)
        .arg("route-decision")
        .arg("--user-utterance")
        .arg(utterance);
    if explicit {
        cmd.arg("--explicit");
    }
    let out = cmd.output().expect("spawn route-decision");
    assert_eq!(
        out.status.code(),
        Some(0),
        "route-decision must always exit 0 (fail-open); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf8 stdout")
}

/// Extract a string field from the helper's compact JSON without serde_json
/// (integration-test crates only see `[dev-dependencies]`, which is `tempfile`
/// only). The output is machine-emitted compact JSON, so `"key":"value"` slices
/// are stable.
fn str_field(json: &str, key: &str) -> String {
    let needle = format!("\"{key}\":\"");
    let start = json
        .find(&needle)
        .unwrap_or_else(|| panic!("missing string field {key:?} in {json:?}"))
        + needle.len();
    let rest = &json[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("unterminated field {key:?} in {json:?}"));
    rest[..end].to_string()
}

/// Extract a boolean field (`"key":true` / `"key":false`) from the compact JSON.
fn bool_field(json: &str, key: &str) -> bool {
    let needle = format!("\"{key}\":");
    let start = json
        .find(&needle)
        .unwrap_or_else(|| panic!("missing bool field {key:?} in {json:?}"))
        + needle.len();
    let rest = &json[start..];
    if rest.starts_with("true") {
        true
    } else if rest.starts_with("false") {
        false
    } else {
        panic!("field {key:?} is not a bool in {json:?}");
    }
}

/// THE AC-6 case: "axhub 말고 vercel 로 배포해" names both targets in one prompt.
/// The decision must be `ask`, and — the ontology differentiator vs the gate
/// test — the SAME output must report both conflict booleans true. Because the
/// SKILL proceeds only on `axhub`, an `ask` here means the preflight blocks the
/// deploy and opens the disambiguation instead.
#[test]
fn axhub_plus_vercel_conflict_is_ask_with_both_keyword_flags() {
    let r = repo(true); // marker present: ambiguity must still win over rule d.
    let cfg = config_home(true);
    let json = route(r.path(), cfg.path(), "axhub 말고 vercel 로 배포해", false);

    assert_eq!(
        str_field(&json, "decision"),
        "ask",
        "axhub + foreign in one prompt must be the ambiguous `ask`"
    );
    // The conflict condition itself — both ontology booleans true in one output.
    assert!(
        bool_field(&json, "axhub_keyword"),
        "axhub_keyword_present must be true for the conflict prompt"
    );
    assert!(
        bool_field(&json, "foreign_keyword"),
        "foreign_keyword_present must be true for the conflict prompt"
    );
    // Preflight-block contract: ask is NOT axhub, so Step 0 does not proceed.
    assert_ne!(
        str_field(&json, "decision"),
        "axhub",
        "ask must not be treated as proceed — the gate opens disambiguation"
    );
}

/// Every one of the six hardcoded foreign keywords, named alongside "axhub" in
/// one prompt, is the same ambiguous conflict → `ask` with both booleans true.
/// The gate test only exercises vercel; this is the full §45-47 sweep.
#[test]
fn every_foreign_keyword_in_conflict_with_axhub_asks() {
    let r = repo(false); // no marker: rule a (ask) precedes the marker rules.
    let cfg = config_home(true);
    for kw in FOREIGN_KEYWORDS {
        let utterance = format!("axhub 말고 {kw} 로 배포해");
        let json = route(r.path(), cfg.path(), &utterance, false);
        assert_eq!(
            str_field(&json, "decision"),
            "ask",
            "axhub + {kw} must be ask; got {json:?}"
        );
        assert!(
            bool_field(&json, "axhub_keyword") && bool_field(&json, "foreign_keyword"),
            "both conflict booleans must be true for axhub + {kw}; got {json:?}"
        );
    }
}

/// The conflict `ask` must be invariant to marker presence AND auth state — the
/// ambiguity cannot collapse into a marker-driven `axhub` (rule d) or an
/// auth-driven branch. Rule a sits above every marker/auth rule, so all four
/// marker×auth combinations stay `ask`.
#[test]
fn conflict_ask_invariant_across_marker_and_auth() {
    for marker in [true, false] {
        let r = repo(marker);
        for authed in [true, false] {
            let cfg = config_home(authed);
            let json = route(
                r.path(),
                cfg.path(),
                "axhub 랑 netlify 중에 골라줘 배포",
                false,
            );
            assert_eq!(
                str_field(&json, "decision"),
                "ask",
                "conflict must stay ask at marker={marker} authed={authed}; got {json:?}"
            );
        }
    }
}

/// Discriminator (one line each — exhaustive single-keyword coverage is AC-3/AC-4
/// scope): the SAME marker/auth context routes axhub-only → `axhub` and
/// foreign-only → `yield`. Proves it is the *co-occurrence*, not an incidental
/// input, that produces `ask`.
#[test]
fn conflict_ask_distinct_from_single_keyword_decisions() {
    let r = repo(false);
    let cfg = config_home(true);
    assert_eq!(
        str_field(
            &route(r.path(), cfg.path(), "axhub 로 배포해", false),
            "decision"
        ),
        "axhub",
        "axhub-only (no foreign) must be axhub, not ask"
    );
    assert_eq!(
        str_field(
            &route(r.path(), cfg.path(), "vercel 로 배포해", false),
            "decision"
        ),
        "yield",
        "foreign-only (no axhub) must be yield, not ask"
    );
}

//! spec 006 AC 15 — the deploy SKILL preflight Step 0 gate blocks axhub deploy
//! when the shared routing decision is `yield`, `ignore`, or `ask`, and only
//! proceeds on `axhub`.
//!
//! The deploy SKILL's Step 0 is bash, so its entry into the single shared
//! routing-decision function (`routing::decide_from_flags`) is the
//! `route-decision` subcommand. This test drives that subcommand exactly as the
//! SKILL does — a `--user-utterance`, an optional `--explicit` slash signal, a
//! real cwd marker walk-up, and a token-file `.exists()` auth probe — and asserts
//! the emitted `.decision`. Because the SKILL proceeds **only** when
//! `decision == "axhub"`, asserting the decision here pins the block/proceed
//! contract Step 0 consumes end to end (the per-decision SKILL branch wiring is
//! pinned separately by the bun test `deploy-preflight-routing-gate.test.ts`).
//!
//! Auth is controlled via `XDG_CONFIG_HOME` (token_present() resolves
//! `$XDG_CONFIG_HOME/axhub-plugin/token`), and the marker via the spawned process
//! cwd — neither spawns the axhub CLI nor triggers token-init (spec §102).

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

/// Build a repo dir under a fresh tempdir. `.git` is always created so the
/// marker walk-up terminates deterministically at this root (cannot escape to a
/// tempdir ancestor); `axhub.yaml` is written only when `marker` is true.
fn repo(marker: bool) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    if marker {
        std::fs::write(tmp.path().join("axhub.yaml"), "app: demo\n").expect("write marker");
    }
    tmp
}

/// A config-home tempdir; when `authed`, it carries the token file that
/// `token_present()` stats. Returned so the dir lives for the spawn's lifetime.
fn config_home(authed: bool) -> tempfile::TempDir {
    let cfg = tempfile::tempdir().expect("cfg tempdir");
    if authed {
        let dir = cfg.path().join("axhub-plugin");
        std::fs::create_dir_all(&dir).expect("mkdir axhub-plugin");
        std::fs::write(dir.join("token"), "axhub_pat_stub\n").expect("write token");
    }
    cfg
}

/// Run `route-decision` the way the SKILL Step 0 does and return the parsed
/// `.decision` string. Asserts exit 0 (fail-open contract).
fn decision(cwd: &Path, config: &Path, utterance: &str, explicit: bool) -> String {
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
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    extract_decision(&stdout)
}

/// Pull the `.decision` string out of the compact JSON without a serde_json
/// dependency (integration-test crates only see `[dev-dependencies]`). The
/// output is machine-emitted compact JSON, so the `"decision":"…"` slice is stable.
fn extract_decision(json: &str) -> String {
    const KEY: &str = "\"decision\":\"";
    let start = json
        .find(KEY)
        .unwrap_or_else(|| panic!("missing .decision in {json:?}"))
        + KEY.len();
    let rest = &json[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("unterminated .decision in {json:?}"));
    rest[..end].to_string()
}

/// AC 15 block row — non-marker repo + implicit "배포해" → `ignore`. The SKILL
/// must NOT proceed; it opens the disambiguation question instead. Auth-invariant
/// (rule e, not the Unknown fallback), so asserted for both auth states.
#[test]
fn non_marker_implicit_blocks_with_ignore() {
    let cfg_unauthed = config_home(false);
    let cfg_authed = config_home(true);
    let r = repo(false);
    assert_eq!(
        decision(r.path(), cfg_unauthed.path(), "배포해", false),
        "ignore",
        "non-marker + implicit (unauthed) must block as ignore"
    );
    assert_eq!(
        decision(r.path(), cfg_authed.path(), "배포해", false),
        "ignore",
        "non-marker + implicit (authed) must still block as ignore (rule e)"
    );
}

/// AC 15 block row — marker present + foreign target named ("vercel에 배포해") →
/// `yield` (named-target-wins beats the marker). The SKILL steps aside without a
/// disambiguation question. Auth-invariant.
#[test]
fn marker_present_foreign_keyword_blocks_with_yield() {
    let r = repo(true);
    for authed in [false, true] {
        let cfg = config_home(authed);
        assert_eq!(
            decision(r.path(), cfg.path(), "vercel에 배포해", false),
            "yield",
            "marker + foreign keyword (authed={authed}) must yield"
        );
    }
}

/// AC 15 block row — both axhub and a foreign target named → `ask`
/// (disambiguation). Marker presence must not collapse the ambiguity.
#[test]
fn axhub_plus_foreign_blocks_with_ask() {
    let cfg = config_home(true);
    for marker in [true, false] {
        let r = repo(marker);
        assert_eq!(
            decision(r.path(), cfg.path(), "axhub 말고 vercel 로 배포해", false),
            "ask",
            "axhub + foreign (marker={marker}) must ask"
        );
    }
}

/// AC 15 proceed row — marker present + implicit "배포해" → `axhub`. The existing
/// axhub.yaml user keeps implicit-deploy (backward-compat); the gate does not block.
#[test]
fn marker_present_implicit_proceeds_with_axhub() {
    let r = repo(true);
    for authed in [false, true] {
        let cfg = config_home(authed);
        assert_eq!(
            decision(r.path(), cfg.path(), "배포해", false),
            "axhub",
            "marker + implicit (authed={authed}) must proceed as axhub"
        );
    }
}

/// explicit-always-works — `--explicit` (the SKILL's model-set slash signal)
/// forces `axhub` even in a non-marker repo for an unauthed user, the worst case
/// for the marker/auth axes. Mirrors how `commands/deploy.md` forwards only
/// `$ARGUMENTS` (no leading `/deploy` token), so the flag is the carrier.
#[test]
fn explicit_flag_proceeds_with_axhub_even_non_marker_unauthed() {
    let r = repo(false);
    let cfg = config_home(false);
    // Control: same args without the explicit flag block (ignore).
    assert_eq!(
        decision(r.path(), cfg.path(), "paydrop", false),
        "ignore",
        "control: non-marker bare arg without explicit must block"
    );
    // The explicit slash signal overrides marker-absence + unauthed → axhub.
    assert_eq!(
        decision(r.path(), cfg.path(), "paydrop", true),
        "axhub",
        "explicit invocation must proceed as axhub regardless of marker/auth"
    );
}

/// A `/deploy` slash still present in the utterance text is detected on its own
/// (without the `--explicit` flag) → `axhub`. Belt-and-suspenders for the case
/// where the slash token survives into `$ARGS`.
#[test]
fn slash_in_utterance_proceeds_without_explicit_flag() {
    let r = repo(false);
    let cfg = config_home(false);
    assert_eq!(
        decision(r.path(), cfg.path(), "/deploy paydrop", false),
        "axhub",
        "a slash left in the utterance must be detected as explicit → axhub"
    );
    // Korean alias /배포 (commands/배포.md) must be detected identically.
    assert_eq!(
        decision(r.path(), cfg.path(), "/배포 paydrop", false),
        "axhub",
        "the Korean /배포 slash must also be detected as explicit → axhub"
    );
}

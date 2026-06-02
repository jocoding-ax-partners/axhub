//! spec 006 (AC 19) — the **bounded double surface**: grace once, then preflight
//! only.
//!
//! Two distinct surfaces can speak to a non-marker authed user who types an
//! implicit deploy request ("배포해"). The **hook surface (grace)**:
//! `prompt-route` emits a one-time migration `systemMessage` (the
//! `maybe_grace_message` nudge). The **preflight surface (disambiguation)**: the
//! deploy SKILL's Step 0 reads the shared routing decision via the
//! `route-decision` subcommand and disambiguates whenever the decision is not
//! `axhub`.
//!
//! spec §43 ("이중 노출 정책") bounds the pair so the user is never double-nudged
//! forever: the grace **educates once per project**, while the preflight
//! disambiguation **blocks every time**. So after the first prompt only the
//! preflight remains — "grace once then preflight only".
//!
//! This is the *composition* lock and is deliberately distinct from its
//! neighbours. AC 11 (`routing_grace_once_per_project_ac11`) tests the grace
//! surface **alone** (once-per-project); AC 15 (`routing_preflight_gate`) tests
//! the preflight surface **alone** (decision = block/proceed).
//! Neither asserts that the two surfaces *hand off*: that exactly when grace goes
//! silent (round 2), the preflight surface is still live. That handoff is the
//! whole of AC 19, so both surfaces are exercised in the **same** scenario here.
//!
//! Why the handoff is the only coherent reading: the hook layer is
//! `systemMessage`-injection only and disambiguation is single-owned by the
//! preflight — there is no cross-layer suppression mechanism, so the preflight
//! cannot be silenced by grace having fired. The preflight therefore fires in
//! **both** rounds; grace is purely a one-time layer on top.
//!
//! Determinism: a tempdir cwd with `.git` and no `axhub.yaml` (→ marker
//! `Absent`), a tempdir `XDG_CONFIG_HOME` carrying the auth token file
//! (`token_present()` reads authed), and a tempdir `XDG_STATE_HOME` (the grace
//! once-marker home). The `route-decision` subcommand is pure
//! (`decide_from_flags`, no `try_consume_once`), so it never touches the grace
//! slot — round 2's silence is attributable solely to the two `prompt-route`
//! calls. To keep that attribution bulletproof the calls are not interleaved:
//! both `prompt-route` rounds run first, then `route-decision` for the
//! persistence check.

use std::path::Path;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

const DEPLOY_PROMPT: &str = "배포해";

/// A non-marker git repo: `.git` present (walk-up terminates here as `Absent`),
/// deliberately no `axhub.yaml`.
fn non_marker_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("repo tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    tmp
}

/// Write a token file under `config_home` so `routing::token_present()` (a pure
/// `.exists()` stat — never bootstrap) reads the user as axhub-authed.
fn make_authed(config_home: &Path) {
    let token = config_home.join("axhub-plugin").join("token");
    std::fs::create_dir_all(token.parent().unwrap()).expect("mkdir token dir");
    std::fs::write(&token, b"axhub_pat_stub\n").expect("write token");
}

/// Drive the **hook surface**: run `prompt-route` with a fully controlled cwd +
/// environment, returning the raw process output.
fn run_prompt_route(prompt: &str, cwd: &Path, config_home: &Path, state_home: &Path) -> Output {
    use std::io::Write;
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": prompt,
    })
    .to_string();

    let mut child = Command::new(bin())
        .arg("prompt-route")
        .current_dir(cwd)
        .env("XDG_CONFIG_HOME", config_home)
        .env("XDG_STATE_HOME", state_home)
        // Keep preflight offline/fast: a non-existent CLI → cli_present=false.
        .env("AXHUB_BIN", "/nonexistent/axhub-binary")
        // Audit writes are irrelevant here and would only clutter the state dir.
        .env("AXHUB_NO_AUDIT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn prompt-route");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait prompt-route")
}

/// The top-level `systemMessage` (the grace channel), if any. `additionalContext`
/// carries the agent-facing preflight and is intentionally ignored here.
fn grace_system_message(output: &Output) -> Option<String> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "prompt-route must fail-open exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.contains("hookSpecificOutput"))
        .unwrap_or("");
    let json: serde_json::Value = serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
    json.get("systemMessage")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Drive the **preflight surface**: run `route-decision` exactly as the deploy
/// SKILL's Step 0 does and return the parsed `.decision`. Asserts the fail-open
/// exit-0 contract.
fn route_decision(cwd: &Path, config_home: &Path, utterance: &str) -> String {
    let out = Command::new(bin())
        .current_dir(cwd)
        .env("XDG_CONFIG_HOME", config_home)
        .arg("route-decision")
        .arg("--user-utterance")
        .arg(utterance)
        .output()
        .expect("spawn route-decision");
    assert_eq!(
        out.status.code(),
        Some(0),
        "route-decision must always exit 0 (fail-open); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    extract_decision(&String::from_utf8(out.stdout).expect("utf8 stdout"))
}

/// Pull the `.decision` string out of the compact machine JSON without a
/// serde_json round-trip on the value (the slice is stable for emitted output).
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

/// THE AC 19 contract — grace once, then preflight only.
///
/// Round 1 (first implicit deploy prompt): the surface is genuinely **double** —
/// the grace `systemMessage` is present AND the preflight decision is `ignore`
/// (so Step 0 would disambiguate). Round 2 (identical prompt, same project): the
/// grace is gone (bounded to one exposure) BUT the preflight decision is STILL
/// `ignore` — the preflight is the persistent surface that remains.
#[test]
fn grace_once_then_preflight_only() {
    let repo = non_marker_repo();
    let config_home = tempfile::tempdir().expect("config home");
    let state_home = tempfile::tempdir().expect("state home");
    make_authed(config_home.path());

    // --- both hook rounds first (no interleave → round-2 silence is solely the
    //     two prompt-route calls, never the pure route-decision probe). ---
    let r1 = run_prompt_route(
        DEPLOY_PROMPT,
        repo.path(),
        config_home.path(),
        state_home.path(),
    );
    let r2 = run_prompt_route(
        DEPLOY_PROMPT,
        repo.path(),
        config_home.path(),
        state_home.path(),
    );

    // Round 1 hook surface: grace present. A single recognizable token is enough
    // (AC 11 owns the full migration-path text); the real discriminator is the
    // presence→absence transition below. Presence here also self-guards against a
    // silently-broken `make_authed` (an unauthed run would be silent and the test
    // would pass for the wrong reason).
    let g1 = grace_system_message(&r1)
        .expect("round 1: authed non-marker implicit deploy must emit the grace systemMessage");
    assert!(
        g1.contains("axhub.yaml"),
        "round 1 systemMessage must be the migration grace nudge: {g1}"
    );

    // Round 2 hook surface: grace silent (bounded to once-per-project).
    assert_eq!(
        grace_system_message(&r2),
        None,
        "round 2: the grace nudge must NOT repeat within the same project"
    );

    // --- preflight surface: the same shared decision both rounds consume. It is
    //     stateless, so its verdict is identical regardless of grace lifecycle. ---
    assert_eq!(
        route_decision(repo.path(), config_home.path(), DEPLOY_PROMPT),
        "ignore",
        "preflight surface must disambiguate (decision=ignore) — this is the \
         surface that persists after grace is consumed"
    );
}

/// The "preflight only" half, isolated: the preflight decision is **invariant**
/// across the entire grace lifecycle — before grace has ever fired, and after it
/// has been consumed. This pins that the preflight is the stable, always-live
/// surface (the one that remains once grace falls silent), not a one-shot like
/// grace.
#[test]
fn preflight_surface_is_invariant_across_grace_lifecycle() {
    let repo = non_marker_repo();
    let config_home = tempfile::tempdir().expect("config home");
    let state_home = tempfile::tempdir().expect("state home");
    make_authed(config_home.path());

    // Before any prompt-route call (grace slot untouched): preflight already
    // disambiguates.
    let before = route_decision(repo.path(), config_home.path(), DEPLOY_PROMPT);

    // Consume the one-time grace slot via a real hook round.
    let hook = run_prompt_route(
        DEPLOY_PROMPT,
        repo.path(),
        config_home.path(),
        state_home.path(),
    );
    assert!(
        grace_system_message(&hook).is_some(),
        "the hook round must consume the grace slot (precondition)"
    );

    // After grace is consumed: preflight decision is unchanged.
    let after = route_decision(repo.path(), config_home.path(), DEPLOY_PROMPT);

    assert_eq!(before, "ignore", "preflight disambiguates before grace fires");
    assert_eq!(
        before, after,
        "preflight decision must be invariant across the grace lifecycle \
         (before={before}, after={after}) — preflight is the persistent surface"
    );
}

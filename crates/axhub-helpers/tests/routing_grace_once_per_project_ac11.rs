//! spec 006 (AC 11) — once-per-project migration **grace warning**.
//!
//! Scenario: an axhub-**authed** developer, in a repo with **no** `axhub.yaml`
//! marker, types an **implicit** deploy request ("배포해"). The shared routing
//! decision is [`RoutingDecision::Ignore`] (rule `e`) so axhub yields the repo —
//! but because this user previously leaned on the implicit nudge, the
//! prompt-route hook emits a **one-time** `systemMessage` explaining the new
//! contract (`/init` or `"axhub 배포"`). It must fire **exactly once per
//! project** (spec §43 "이중 노출 정책": grace educates once; the deploy
//! preflight blocks every time thereafter).
//!
//! This drives the real `prompt-route` binary end to end so the assertion covers
//! the live wiring (decision → `maybe_grace_message` → systemMessage), not just
//! the pure predicate (which the `grace` unit tests already cover). The four
//! conditions are isolated by discriminators so a green result can only mean the
//! gate is correct:
//!   1. **first authed deploy prompt** → systemMessage present (+ migration text),
//!   2. **second identical prompt, same project** → NO systemMessage (once-only),
//!   3. **unauthed** (no token file) → NO systemMessage (zero-footprint),
//!   4. **non-deploy prompt** while authed → NO systemMessage (intent gate).
//!
//! Determinism: the child process gets a tempdir cwd containing `.git` and no
//! `axhub.yaml` (→ `MarkerStatus::Absent`), plus tempdir `XDG_CONFIG_HOME`
//! (token-file presence = authed) and `XDG_STATE_HOME` (the once-marker home).
//! Nothing reads the developer's real token or state.

use std::path::Path;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

/// Run `prompt-route` with a fully controlled cwd + environment.
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

/// A non-marker git repo: `.git` present (walk-up terminates here as `Absent`),
/// deliberately no `axhub.yaml`.
fn non_marker_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("repo tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    tmp
}

/// Write a token file under `config_home` so `routing::token_present()` (a pure
/// `.exists()` stat) reads the user as axhub-authed.
fn make_authed(config_home: &Path) {
    let token = config_home.join("axhub-plugin").join("token");
    std::fs::create_dir_all(token.parent().unwrap()).expect("mkdir token dir");
    std::fs::write(&token, b"fake-delegation-token\n").expect("write token");
}

/// The top-level `systemMessage`, if any. Newer high-risk intent nudges share
/// this channel with the migration grace, so this file checks the distinctive
/// grace text instead of equating every systemMessage with grace.
fn system_message(output: &Output) -> Option<String> {
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

fn contains_grace_nudge(message: Option<&str>) -> bool {
    message
        .map(|msg| {
            msg.contains("axhub.yaml") && msg.contains("/init") && msg.contains("axhub 배포")
        })
        .unwrap_or(false)
}

const DEPLOY_PROMPT: &str = "배포해";

/// The headline once-per-project contract: first authed implicit deploy prompt
/// shows the grace nudge; an identical second prompt in the SAME project does not.
#[test]
fn grace_fires_once_per_project_for_authed_no_marker_implicit_deploy() {
    let repo = non_marker_repo();
    let config_home = tempfile::tempdir().expect("config home");
    let state_home = tempfile::tempdir().expect("state home");
    make_authed(config_home.path());

    // First prompt → grace nudge present, names both recovery paths.
    let first = run_prompt_route(
        DEPLOY_PROMPT,
        repo.path(),
        config_home.path(),
        state_home.path(),
    );
    let msg =
        system_message(&first).expect("first authed deploy prompt must emit grace systemMessage");
    assert!(
        contains_grace_nudge(Some(&msg)),
        "grace must name the migration paths: {msg}"
    );

    // Second identical prompt, same project → silent (once-per-project).
    let second = run_prompt_route(
        DEPLOY_PROMPT,
        repo.path(),
        config_home.path(),
        state_home.path(),
    );
    let second_message = system_message(&second);
    assert!(
        !contains_grace_nudge(second_message.as_deref()),
        "the grace nudge must NOT repeat within the same project; got {second_message:?}"
    );
}

/// Discriminator: an **unauthed** user (no token file) in the same non-marker
/// repo with the same deploy prompt gets NO nudge — zero-footprint for
/// non-axhub users is preserved (the auth gate is load-bearing, not incidental).
#[test]
fn grace_silent_for_unauthed_user() {
    let repo = non_marker_repo();
    let config_home = tempfile::tempdir().expect("config home"); // no token written
    let state_home = tempfile::tempdir().expect("state home");

    let output = run_prompt_route(
        DEPLOY_PROMPT,
        repo.path(),
        config_home.path(),
        state_home.path(),
    );
    let message = system_message(&output);
    assert!(
        !contains_grace_nudge(message.as_deref()),
        "an unauthed user must never see the migration grace nudge; got {message:?}"
    );
}

/// Discriminator: an authed user whose prompt is NOT a deploy request gets no
/// nudge — the coarse deploy-intent gate is what makes the nudge deploy-specific
/// (so "안녕" does not burn the once-slot or spam the user).
#[test]
fn grace_silent_for_non_deploy_prompt() {
    let repo = non_marker_repo();
    let config_home = tempfile::tempdir().expect("config home");
    let state_home = tempfile::tempdir().expect("state home");
    make_authed(config_home.path());

    let output = run_prompt_route(
        "안녕하세요",
        repo.path(),
        config_home.path(),
        state_home.path(),
    );
    let message = system_message(&output);
    assert!(
        !contains_grace_nudge(message.as_deref()),
        "a non-deploy prompt must not trigger the deploy migration nudge; got {message:?}"
    );
}

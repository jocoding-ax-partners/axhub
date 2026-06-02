// Phase 25 PR 25.2 — Hook safety CLI integration tests.
//
// Verifies that the four axhub hook entry points (session-start, preauth-check,
// prompt-route, classify-exit) honor `AXHUB_DISABLE_HOOKS`,
// `AXHUB_DISABLE_HOOK=<csv>`, and the legacy `DISABLE_AXHUB` alias per the
// kill-switch precedence rules in `docs/HOOKS.md`. Each test spawns the
// helper binary the same way `cli_e2e.rs` does so we exercise the real
// dispatch path, not the unit-test mocks.

use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_stdin(args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Clear inherited env so host shell vars (e.g. AXHUB_TELEMETRY) don't
    // leak into our matrix and falsify "default" baselines.
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");

    // Sandbox audit + telemetry writes for prompt-route.
    let state_dir = tempfile::tempdir().unwrap();
    command.env("XDG_STATE_HOME", state_dir.path());
    command.env("AXHUB_NO_AUDIT", "1");

    for (k, v) in envs {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    match child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::BrokenPipe => {}
        Err(err) => panic!("failed to write child stdin: {err}"),
    }
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

// --- session-start --------------------------------------------------------

#[test]
fn session_start_baseline_emits_welcome_lines() {
    let out = run_stdin(&["session-start"], "", &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        s.contains("axhub 준비됐어요"),
        "baseline session-start should emit welcome line, got: {s}"
    );
}

#[test]
fn session_start_global_kill_switch_skips_silently() {
    let out = run_stdin(&["session-start"], "", &[("AXHUB_DISABLE_HOOKS", "1")]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert_eq!(
        s.trim(),
        "{}",
        "expected empty JSON pass signal, got: {s:?}"
    );
}

#[test]
fn session_start_per_hook_kill_switch_skips() {
    let out = run_stdin(
        &["session-start"],
        "",
        &[("AXHUB_DISABLE_HOOK", "session-start , other")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn session_start_per_hook_csv_without_match_runs_normally() {
    let out = run_stdin(
        &["session-start"],
        "",
        &[("AXHUB_DISABLE_HOOK", "preauth-check,prompt-route")],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("axhub 준비됐어요"));
}

// --- legacy alias ---------------------------------------------------------

#[test]
fn legacy_disable_axhub_skips_with_deprecation_warning() {
    let out = run_stdin(&["session-start"], "", &[("DISABLE_AXHUB", "1")]);
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
    let err = stderr(&out);
    assert!(
        err.contains("DISABLE_AXHUB") && err.contains("deprecated"),
        "expected legacy deprecation warning on stderr, got: {err:?}"
    );
}

#[test]
fn legacy_alias_loses_to_canonical_global() {
    // global canonical present → skip path, no spurious legacy warning.
    let out = run_stdin(
        &["session-start"],
        "",
        &[("AXHUB_DISABLE_HOOKS", "1"), ("DISABLE_AXHUB", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
    // canonical short-circuits before legacy check → no warning.
    let err = stderr(&out);
    assert!(
        !err.contains("deprecated"),
        "canonical should short-circuit legacy warning, got stderr: {err:?}"
    );
}

// --- preauth-check --------------------------------------------------------

#[test]
fn preauth_check_kill_switch_returns_allow() {
    let out = run_stdin(
        &["preauth-check"],
        r#"{"tool_name":"Bash","tool_input":{"command":"axhub deploy create"}}"#,
        &[("AXHUB_DISABLE_HOOKS", "1")],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        s.contains("\"permissionDecision\":\"allow\""),
        "preauth-check skip should still emit allow, got: {s}"
    );
}

#[test]
fn preauth_check_per_hook_csv_skips_only_listed() {
    let out = run_stdin(
        &["preauth-check"],
        r#"{"tool_name":"Bash","tool_input":{"command":"axhub deploy create"}}"#,
        &[("AXHUB_DISABLE_HOOK", "preauth-check")],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("\"permissionDecision\":\"allow\""));
}

// --- prompt-route ---------------------------------------------------------

#[test]
fn prompt_route_kill_switch_skips_audit() {
    let out = run_stdin(
        &["prompt-route"],
        r#"{"prompt":"axhub 배포해줘"}"#,
        &[("AXHUB_DISABLE_HOOKS", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

// --- classify-exit --------------------------------------------------------

#[test]
fn classify_exit_kill_switch_skips() {
    let out = run_stdin(
        &["classify-exit"],
        r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":64,"stdout":"oops"}}"#,
        &[("AXHUB_DISABLE_HOOKS", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn classify_exit_per_hook_csv_skips() {
    let out = run_stdin(
        &["classify-exit"],
        r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":64,"stdout":"oops"}}"#,
        &[("AXHUB_DISABLE_HOOK", "classify-exit")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

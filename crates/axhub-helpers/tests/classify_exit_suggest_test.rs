// Phase 25 PR 25.7 — classify-exit verify/trace auto-suggest coverage.
//
// Mirrors `cli_e2e.rs` PostToolUse payload test pattern. Each case feeds a
// JSON envelope to `axhub-helpers classify-exit` and asserts:
//   - the verify/trace nudge appears or not (depending on command + exit_code)
//   - the natural-language Korean phrase comes BEFORE the slash command (D4)
//   - the kill-switch env vars from PR 25.2 silence the suggest entirely

use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_classify_exit(stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(["classify-exit"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Clear inherited kill-switch envs so a host environment never silences
    // the suggest paths under test.
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");

    for (k, v) in envs {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    let write_result = child.stdin.as_mut().unwrap().write_all(stdin.as_bytes());
    if let Err(error) = write_result {
        assert_eq!(
            error.kind(),
            ErrorKind::BrokenPipe,
            "stdin write failed with unexpected error: {error}"
        );
    }
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

// ---------------------------------------------------------------------------
// verify suggest (axhub deploy create + exit 0)
// ---------------------------------------------------------------------------

#[test]
fn deploy_create_success_suggests_verify_with_nl_trigger_first() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create --json"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("배포 완료"), "got: {s}");
    assert!(s.contains("\\\"확인해\\\""), "nl-trigger missing: {s}");
    assert!(
        !s.contains("/axhub:verify"),
        "unregistered slash command leaked: {s}"
    );
}

#[test]
fn recover_success_suggests_verify_with_nl_trigger_first() {
    let payload = r#"{"tool_input":{"command":"axhub recover --app-id 42"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("복구 완료"), "got: {s}");
    assert!(s.contains("확인해"));
    assert!(s.contains("확인해"));
    assert!(
        !s.contains("/axhub:verify"),
        "unregistered slash command leaked: {s}"
    );
}

// ---------------------------------------------------------------------------
// trace suggest (axhub deploy create + exit 64..=68)
// ---------------------------------------------------------------------------

#[test]
fn deploy_create_validation_failure_suggests_trace() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":64,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("배포 실패"), "got: {s}");
    assert!(s.contains("왜 실패했어"));
    assert!(s.contains("왜 실패했어"));
    assert!(
        !s.contains("/axhub:trace"),
        "unregistered slash command leaked: {s}"
    );
    // empathy catalog entry should still be present alongside the suggest.
    assert!(
        s.contains("배포는 시작") || s.contains("권한"),
        "empathy catalog entry must coexist with suggest: {s}"
    );
}

#[test]
fn deploy_create_auth_failure_suggests_trace() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":65,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("왜 실패했어"));
    assert!(s.contains("왜 실패했어"));
    assert!(
        !s.contains("/axhub:trace"),
        "unregistered slash command leaked: {s}"
    );
}

#[test]
fn deploy_create_rate_limit_suggests_trace() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":68,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    let s = stdout(&out);
    assert!(s.contains("왜 실패했어"));
    assert!(
        !s.contains("/axhub:trace"),
        "unregistered slash command leaked: {s}"
    );
}

// ---------------------------------------------------------------------------
// no-suggest paths
// ---------------------------------------------------------------------------

#[test]
fn non_axhub_command_emits_empty_json() {
    let payload =
        r#"{"tool_input":{"command":"ls -la"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn axhub_status_success_emits_empty_json_no_suggest() {
    let payload =
        r#"{"tool_input":{"command":"axhub status"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    // Not deploy-create, exit 0, no suggest match → empty.
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn axhub_other_failure_emits_empathy_no_trace_suggest() {
    let payload =
        r#"{"tool_input":{"command":"axhub status"},"tool_response":{"exit_code":65,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[]);
    let s = stdout(&out);
    // Exit 65 catalog entry shows (auth empathy), but trace suggest is
    // gated on `axhub deploy create` specifically.
    assert!(
        !s.contains("/axhub:trace"),
        "non-deploy must not get trace suggest: {s}"
    );
}

// ---------------------------------------------------------------------------
// kill switch (PR 25.2 inline mirror)
// ---------------------------------------------------------------------------

#[test]
fn global_kill_switch_silences_suggest() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[("AXHUB_DISABLE_HOOKS", "1")]);
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn per_hook_kill_switch_silences_suggest() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[("AXHUB_DISABLE_HOOK", "classify-exit")]);
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn legacy_alias_silences_suggest() {
    let payload = r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":0,"stdout":""}}"#;
    let out = run_classify_exit(payload, &[("DISABLE_AXHUB", "1")]);
    assert_eq!(stdout(&out).trim(), "{}");
}

use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

use axhub_helpers::verify_deploy_artifact::{verify_user_app_artifact, VerifyOutcome};
use serde_json::Value;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_verify_hook(stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(["verify-deploy-artifact"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    for (key, value) in envs {
        command.env(key, value);
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

fn stdout_json(out: &Output) -> Value {
    serde_json::from_str(stdout(out).trim()).expect("stdout should be JSON")
}

#[test]
fn verifier_accepts_ts_parity_happy_paths() {
    let result = verify_user_app_artifact(
        &serde_json::json!({
            "manifest_hash": "A".repeat(64),
            "state": "success",
            "url": "http://app.example.com",
            "deploy_id": "dep_123"
        })
        .to_string(),
    );

    assert_eq!(
        result.outcome,
        VerifyOutcome::Confirmed,
        "uppercase sha + success + deploy_id should be Confirmed"
    );
    assert!(result.violations.is_empty());
}

#[test]
fn verifier_flags_ts_parity_violations() {
    let result = verify_user_app_artifact(
        &serde_json::json!({
            "manifest_hash": "not-a-sha256",
            "state": "rolled_back",
            "url": "ftp://wrong.example.com",
            "deployment_id": "   "
        })
        .to_string(),
    );

    assert_eq!(result.outcome, VerifyOutcome::Violation);
    let violations = result.violations.join("\n");
    assert!(violations.contains("sha256 hex"));
    assert!(violations.contains("rolled_back"));
    assert!(violations.contains("http(s)"));
    assert!(violations.contains("deployment_id"));
}

#[test]
fn verifier_downgrades_non_object_or_non_json_stdout_to_unconfirmed() {
    // No parseable object → no signal to confirm success. This is advisory
    // (Unconfirmed), never a hard Violation that would break a normal deploy.
    for stdout in [
        "Deployment started\nLive at https://app.example.com\n",
        "[1,2,3]",
        "null",
    ] {
        let result = verify_user_app_artifact(stdout);
        assert_eq!(
            result.outcome,
            VerifyOutcome::Unconfirmed,
            "{stdout:?} should be Unconfirmed, not a violation"
        );
        assert!(result.violations.is_empty());
        assert!(!result.advisories.is_empty());
    }
}

#[test]
fn cli_happy_path_is_silent_post_tool_use_hook() {
    let payload = serde_json::json!({
        "tool_input": { "command": "axhub deploy create --json" },
        "tool_response": {
            "exit_code": 0,
            "stdout": serde_json::json!({
                "manifest_hash": "b".repeat(64),
                "state": "live",
                "url": "https://app.example.com",
                "deployment_id": "dep_ok"
            }).to_string()
        }
    });
    let out = run_verify_hook(&payload.to_string(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out).trim(), "");
}

#[test]
fn cli_violation_emits_korean_system_message_and_post_tool_context() {
    let payload = serde_json::json!({
        "tool_input": { "command": "  axhub  deploy   create; echo done" },
        "tool_response": {
            "exit_code": 0,
            "stdout": serde_json::json!({ "state": "rolled_back" }).to_string()
        }
    });
    let out = run_verify_hook(&payload.to_string(), &[]);
    assert_eq!(out.status.code(), Some(0));

    let value = stdout_json(&out);
    assert!(value["systemMessage"]
        .as_str()
        .unwrap_or_default()
        .contains("배포 artifact 검증에서 의심 신호를 발견했어요"));
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    let context = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap_or_default();
    assert!(context.contains("<axhub-deploy-verify>"));
    assert!(context.contains("Observed:"));
    assert!(context.contains("Suggested:"));
    assert!(context.contains("Skip: AXHUB_DISABLE_HOOK=verify-deploy-artifact"));
    assert!(context.contains("rolled_back"));
}

#[test]
fn cli_fail_opens_for_non_deploy_failure_or_malformed_payload() {
    let cases = [
        serde_json::json!({
            "tool_input": { "command": "ls -la" },
            "tool_response": { "exit_code": 0, "stdout": "{\"state\":\"rolled_back\"}" }
        })
        .to_string(),
        serde_json::json!({
            "tool_input": { "command": "axhub deploy created" },
            "tool_response": { "exit_code": 0, "stdout": "{\"state\":\"rolled_back\"}" }
        })
        .to_string(),
        serde_json::json!({
            "tool_input": { "command": "axhub deploy create" },
            "tool_response": { "exit_code": 64, "stdout": "{\"state\":\"rolled_back\"}" }
        })
        .to_string(),
        "not json at all".to_string(),
        serde_json::json!({
            "tool_input": { "command": "axhub deploy create" }
        })
        .to_string(),
    ];

    for payload in cases {
        let out = run_verify_hook(&payload, &[]);
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(stdout(&out).trim(), "");
    }
}

#[test]
fn cli_honors_canonical_and_legacy_kill_switches() {
    let payload = serde_json::json!({
        "tool_input": { "command": "axhub deploy create" },
        "tool_response": {
            "exit_code": 0,
            "stdout": serde_json::json!({ "state": "rolled_back" }).to_string()
        }
    })
    .to_string();

    for envs in [
        [("AXHUB_DISABLE_HOOKS", "1")].as_slice(),
        [("AXHUB_DISABLE_HOOK", "verify-deploy-artifact,other")].as_slice(),
        [(
            "AXHUB_DISABLE_HOOK",
            "post-tool-verify-deploy-artifacts,other",
        )]
        .as_slice(),
        [("DISABLE_AXHUB", "1")].as_slice(),
    ] {
        let out = run_verify_hook(&payload, envs);
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(stdout(&out).trim(), "");
    }
}

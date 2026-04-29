use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn run_stdin(args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    for (k, v) in envs {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn cli_version_help_redact_and_classify_work() {
    let version = run(&["version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("axhub-helpers"));

    let help = run(&["help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Subcommands"));
    assert!(String::from_utf8_lossy(&help.stdout).contains("prompt-route"));

    let mut child = Command::new(bin())
        .arg("redact")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"Bearer abcdef1234567890abcdef")
        .unwrap();
    let redacted = child.wait_with_output().unwrap();
    assert!(redacted.status.success());
    assert_eq!(String::from_utf8_lossy(&redacted.stdout), "Bearer ***");

    let classified = run(&["classify-exit", "--exit-code", "65", "--stdout", "{}"]);
    assert!(classified.status.success());
    assert!(String::from_utf8_lossy(&classified.stdout).contains("로그인이 만료"));
}

#[cfg(unix)]
#[test]
fn cli_prompt_route_injects_doctor_context_for_version_skew() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.0.5 (commit fake, built fake, fake)"
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ] && [ "$3" = "--json" ]; then
  echo '{"user_email":"test@jocodingax.ai","user_id":1,"expires_at":"2026-04-29T00:00:00Z","scopes":["read"]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"환경 점검해"}"#,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("UserPromptSubmit"));
    assert!(stdout.contains("axhub doctor"));
    assert!(stdout.contains("버전 확인"));
    assert!(stdout.contains("오래된 버전"));
    assert!(stdout.contains("업그레이드"));
}

#[test]
fn cli_usage_preflight_resolve_list_and_session_start_paths_are_stable() {
    let no_args = run(&[]);
    assert_eq!(no_args.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&no_args.stderr).contains("Usage"));

    let unknown = run(&["unknown-subcommand"]);
    assert_eq!(unknown.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown subcommand"));

    let missing_app = run(&["list-deployments"]);
    assert_eq!(missing_app.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&missing_app.stderr).contains("--app-id"));

    let invalid_app = Command::new(bin())
        .args(["list-deployments", "--app", "paydrop"])
        .env("AXHUB_TOKEN", "axhub_pat_abcdefghijklmnop")
        .env("AXHUB_ENDPOINT", "https://example.test")
        .output()
        .unwrap();
    assert_eq!(invalid_app.status.code(), Some(67));
    assert!(String::from_utf8_lossy(&invalid_app.stdout).contains("validation.app_id_invalid"));

    let preflight = Command::new(bin())
        .arg("preflight")
        .env("AXHUB_BIN", "/definitely-not-axhub")
        .output()
        .unwrap();
    assert_eq!(preflight.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&preflight.stdout).contains(r#""cli_present":false"#));

    let resolve = Command::new(bin())
        .args(["resolve", "--user-utterance", "paydrop"])
        .env("AXHUB_BIN", "/definitely-not-axhub")
        .output()
        .unwrap();
    assert_eq!(resolve.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&resolve.stdout).contains("auth_parse_error"));

    let session = run(&["session-start"]);
    assert_eq!(session.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&session.stdout).contains("Rust runtime"));
}

#[test]
fn cli_consent_and_preauth_e2e_preserve_permission_contract() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state").display().to_string();
    let runtime = temp.path().join("runtime").display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "cli-e2e-session"),
    ];
    let binding = serde_json::json!({
        "tool_call_id":"cli-e2e-session:tc-1",
        "action":"deploy_create",
        "app_id":"paydrop",
        "profile":"prod",
        "branch":"main",
        "commit_sha":"abc123"
    })
    .to_string();

    let minted = run_stdin(&["consent-mint"], &binding, &envs);
    assert_eq!(minted.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&minted.stdout).contains("token_id"));

    let verified = run_stdin(&["consent-verify"], &binding, &envs);
    assert_eq!(verified.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&verified.stdout).contains(r#""valid":true"#));

    let allowed_deploy = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-1","tool_name":"Bash","tool_input":{"command":"axhub deploy create --app paydrop --profile prod --branch main --commit abc123"}}"#,
        &envs,
    );
    assert_eq!(allowed_deploy.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&allowed_deploy.stdout).contains("permissionDecision\":\"allow")
    );

    let wrong = binding.replace("\"paydrop\"", "\"otherapp\"");
    let rejected = run_stdin(&["consent-verify"], &wrong, &envs);
    assert_eq!(rejected.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("binding_mismatch:app_id"));

    let non_bash = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-2","tool_name":"Edit","tool_input":{}}"#,
        &envs,
    );
    assert_eq!(non_bash.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&non_bash.stdout).contains("permissionDecision\":\"allow"));

    let read_only = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-3","tool_name":"Bash","tool_input":{"command":"axhub deploy logs --app paydrop"}}"#,
        &envs,
    );
    assert_eq!(read_only.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&read_only.stdout).contains("permissionDecision\":\"allow"));

    let destructive_without_token = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-deny","tool_name":"Bash","tool_input":{"command":"axhub deploy create --app paydrop --branch main --commit abc123"}}"#,
        &envs,
    );
    assert_eq!(destructive_without_token.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&destructive_without_token.stdout)
        .contains("permissionDecision\":\"deny"));

    let identity_without_token = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-login","tool_name":"Bash","tool_input":{"command":"axhub auth login --profile prod"}}"#,
        &envs,
    );
    assert_eq!(identity_without_token.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&identity_without_token.stdout)
        .contains("permissionDecision\":\"deny"));
}

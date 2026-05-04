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

#[cfg(unix)]
#[test]
fn cli_prompt_route_injects_axhub_skill_contexts() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.1.0 (commit fake, built fake, fake)"
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

    let cases = [
        ("결제 앱 만들어줘", "skills/init/SKILL.md", "init template"),
        (
            "Next.js 앱 만들어줘",
            "skills/init/SKILL.md",
            "init template",
        ),
        (
            "axhub.yaml 만들어줘",
            "skills/init/SKILL.md",
            "init template",
        ),
        ("환경변수 뭐 있어?", "skills/env/SKILL.md", "env var"),
        ("환경 변수 확인", "skills/env/SKILL.md", "env var"),
        ("회사 endpoint 바꿔", "skills/profile/SKILL.md", "profile"),
        ("profile current", "skills/profile/SKILL.md", "profile"),
        ("GitHub repo 연결해", "skills/github/SKILL.md", "GitHub"),
        ("결과 봐", "skills/open/SKILL.md", "브라우저"),
        (
            "axhub 뭐 새로 나왔어",
            "skills/whatsnew/SKILL.md",
            "release notes",
        ),
        ("배포해", "skills/deploy/SKILL.md", "bun run release"),
        (
            "내 axhub 앱 목록 보여줘",
            "skills/apps/SKILL.md",
            "팀 scope",
        ),
        ("앱 등록해", "skills/apps/SKILL.md", "팀 scope"),
        (
            "axhub 앱이 어떤 API 쓸 수 있는지 보여줘",
            "skills/apis/SKILL.md",
            "현재 앱",
        ),
        (
            "axhub 에 누구로 로그인돼있어",
            "skills/auth/SKILL.md",
            "identity",
        ),
        ("로그 보여줘", "skills/logs/SKILL.md", "빌드 로그"),
        ("배포 상태 봐", "skills/status/SKILL.md", "진행 상태"),
        ("방금 거 되돌려", "skills/recover/SKILL.md", "직전 안정"),
        ("axhub 새 버전 있어", "skills/update/SKILL.md", "CLI 버전"),
        (
            "axhub 플러그인 업데이트",
            "skills/upgrade/SKILL.md",
            "플러그인 업그레이드",
        ),
        ("axhub 좀 도와줘", "skills/clarify/SKILL.md", "선택지"),
    ];

    for (prompt, skill_path, expected) in cases {
        let input = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": prompt,
        })
        .to_string();
        let output = run_stdin(
            &["prompt-route"],
            &input,
            &[("AXHUB_BIN", axhub.to_str().unwrap())],
        );
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("UserPromptSubmit"), "{stdout}");
        assert!(stdout.contains(skill_path), "{stdout}");
        assert!(stdout.contains(expected), "{stdout}");
    }

    let no_route = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"오늘 날씨 알려줘"}"#,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(no_route.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&no_route.stdout).trim(), "{}");

    let clarify_environment = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"환경"}"#,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(clarify_environment.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&clarify_environment.stdout);
    assert!(stdout.contains("skills/clarify/SKILL.md"), "{stdout}");

    let doctor_environment = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"환경 점검해"}"#,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(doctor_environment.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&doctor_environment.stdout);
    assert!(stdout.contains("skills/doctor/SKILL.md"), "{stdout}");
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
        "commit_sha":"abc123",
        "context": {}
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

    let pending_envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
    ];
    let pending_binding = serde_json::json!({
        "tool_call_id":"pending",
        "action":"deploy_create",
        "app_id":"paydrop",
        "profile":"prod",
        "branch":"main",
        "commit_sha":"def456",
        "context": {}
    })
    .to_string();
    let pending_minted = run_stdin(&["consent-mint"], &pending_binding, &pending_envs);
    assert_eq!(pending_minted.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&pending_minted.stdout).contains("consent-pending-"));
    let pending_allowed = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"actual-claude-session","tool_call_id":"toolu_actual","tool_name":"Bash","tool_input":{"command":"axhub deploy create --app paydrop --profile prod --branch main --commit def456"}}"#,
        &pending_envs,
    );
    assert_eq!(pending_allowed.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&pending_allowed.stdout).contains("permissionDecision\":\"allow")
    );
    let pending_reused = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"actual-claude-session","tool_call_id":"toolu_actual_2","tool_name":"Bash","tool_input":{"command":"axhub deploy create --app paydrop --profile prod --branch main --commit def456"}}"#,
        &pending_envs,
    );
    assert_eq!(pending_reused.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&pending_reused.stdout).contains("permissionDecision\":\"deny"));

    let pending_auth_binding = serde_json::json!({
        "tool_call_id":"pending",
        "action":"auth_login",
        "app_id":"_",
        "profile":"default",
        "branch":"_",
        "commit_sha":"_",
        "context": {}
    })
    .to_string();
    let pending_auth_minted = run_stdin(&["consent-mint"], &pending_auth_binding, &pending_envs);
    assert_eq!(pending_auth_minted.status.code(), Some(0));
    let pending_auth_allowed = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"actual-claude-session","tool_call_id":"toolu_auth","tool_name":"Bash","tool_input":{"command":"axhub auth login"}}"#,
        &pending_envs,
    );
    assert_eq!(pending_auth_allowed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&pending_auth_allowed.stdout)
        .contains("permissionDecision\":\"allow"));

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
    assert_eq!(destructive_without_token.status.code(), Some(0));
    let deny_stdout = String::from_utf8_lossy(&destructive_without_token.stdout);
    assert!(deny_stdout.contains("permissionDecision\":\"deny"));
    assert!(deny_stdout.contains("사전 승인"));

    let identity_without_token = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-login","tool_name":"Bash","tool_input":{"command":"axhub auth login --profile prod"}}"#,
        &envs,
    );
    assert_eq!(identity_without_token.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&identity_without_token.stdout)
        .contains("permissionDecision\":\"deny"));

    let env_binding = serde_json::json!({
        "tool_call_id":"cli-e2e-session:tc-env",
        "action":"env_set",
        "app_id":"paydrop",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"key":"DATABASE_URL"}
    })
    .to_string();
    let minted = run_stdin(&["consent-mint"], &env_binding, &envs);
    assert_eq!(minted.status.code(), Some(0));
    let env_allowed = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-env","tool_name":"Bash","tool_input":{"command":"printf %s \"$DATABASE_URL\" | axhub env set DATABASE_URL --app paydrop --from-stdin --json"}}"#,
        &envs,
    );
    assert_eq!(env_allowed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&env_allowed.stdout).contains("permissionDecision\":\"allow"));

    let cancel_without_token = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-cancel","tool_name":"Bash","tool_input":{"command":"axhub deploy cancel dep_123 --app paydrop --json"}}"#,
        &envs,
    );
    assert_eq!(cancel_without_token.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&cancel_without_token.stdout)
        .contains("permissionDecision\":\"deny"));
}

use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn write_stdin_allowing_early_exit(writer: &mut impl Write, stdin: &str) {
    match writer.write_all(stdin.as_bytes()) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::BrokenPipe => {}
        Err(err) => panic!("failed to write child stdin: {err}"),
    }
}

fn run_stdin(args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let needs_prompt_route_sandbox = args.contains(&"prompt-route")
        && !envs
            .iter()
            .any(|(key, _)| *key == "XDG_STATE_HOME" || *key == "AXHUB_NO_AUDIT");
    let prompt_route_state = if needs_prompt_route_sandbox {
        Some(tempfile::tempdir().unwrap())
    } else {
        None
    };
    if let Some(state) = &prompt_route_state {
        command.env("XDG_STATE_HOME", state.path());
    }
    for (k, v) in envs {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    write_stdin_allowing_early_exit(child.stdin.as_mut().unwrap(), stdin);
    child.wait_with_output().unwrap()
}

fn assert_no_consent_side_effects(state_dir: &Path, runtime_dir: &Path) {
    assert!(!state_dir.exists());
    assert!(!runtime_dir.exists());
}

fn run_in_dir(args: &[&str], cwd: &Path) -> Output {
    Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn run_in_dir_env(args: &[&str], cwd: &Path, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command.args(args).current_dir(cwd);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().unwrap()
}

fn run_stdin_in_dir(args: &[&str], stdin: &str, cwd: &Path) -> Output {
    let mut child = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    write_stdin_allowing_early_exit(child.stdin.as_mut().unwrap(), stdin);
    child.wait_with_output().unwrap()
}

fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "stdout must be json: {err}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_output_does_not_contain(output: &Output, needle: &str) {
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains(needle),
        "stdout leaked {needle}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains(needle),
        "stderr leaked {needle}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn record_apps_create_success(cwd: &Path, plan: &serde_json::Value) -> Output {
    let envelope = serde_json::json!({
        "schema_version": "bootstrap-record/v1",
        "pending_action_id": plan["pending_action_id"],
        "pending_action_hash": plan["pending_action_hash"],
        "command_argv": plan["command"],
        "exit_code": 0,
        "stdout_json": serde_json::from_str::<serde_json::Value>(include_str!("fixtures/bootstrap/apps_create.success.v1.json")).unwrap(),
        "stderr": ""
    });
    run_stdin_in_dir(
        &["bootstrap", "--record", "apps_create", "--json"],
        &envelope.to_string(),
        cwd,
    )
}

fn init_git_with_commit(cwd: &Path) {
    assert!(Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(cwd)
        .output()
        .unwrap()
        .status
        .success());
    for args in [
        ["config", "user.email", "test@example.com"],
        ["config", "user.name", "Axhub Test"],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap()
            .status
            .success());
    }
    assert!(Command::new("git")
        .args(["add", "apphub.yaml", ".gitignore"])
        .current_dir(cwd)
        .output()
        .unwrap()
        .status
        .success());
    assert!(Command::new("git")
        .args(["commit", "-m", "init: test"])
        .current_dir(cwd)
        .output()
        .unwrap()
        .status
        .success());
}

fn write_manifest(dir: &Path) {
    std::fs::write(
        dir.join("apphub.yaml"),
        "name: Paydrop\nslug: paydrop\nframework: nextjs\n",
    )
    .unwrap();
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
    assert!(String::from_utf8_lossy(&help.stdout).contains("bootstrap"));
    assert!(String::from_utf8_lossy(&help.stdout).contains("token-init"));
    assert!(String::from_utf8_lossy(&help.stdout).contains("token-import"));

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

#[test]
fn cli_classify_exit_post_tool_use_payload_branches() {
    let non_axhub = serde_json::json!({
        "tool_input": {"command": "echo hello"},
        "tool_response": {"exit_code": 65, "stdout": "{}"}
    });
    let output = run_stdin(&["classify-exit"], &non_axhub.to_string(), &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_json(&output), serde_json::json!({}));

    let safe_success = serde_json::json!({
        "tool_input": {"command": "axhub apps list --json"},
        "tool_response": {"exit_code": 0, "stdout": "[]"}
    });
    let output = run_stdin(&["classify-exit"], &safe_success.to_string(), &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_json(&output), serde_json::json!({}));

    let deploy_failure = serde_json::json!({
        "tool_input": {"command": "axhub deploy create --app paydrop --json"},
        "tool_response": {"exit_code": 65, "stdout": "{}"}
    });
    let output = run_stdin(&["classify-exit"], &deploy_failure.to_string(), &[]);
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    let msg = json["systemMessage"].as_str().unwrap();
    assert!(msg.contains("로그인이 만료"), "{msg}");
    assert!(msg.contains("선택:"), "{msg}");
}

#[test]
fn cli_token_init_uses_env_fallback_and_writes_plugin_token_file() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_config = temp.path().join("xdg-config");
    let xdg_config_s = xdg_config.to_str().unwrap();
    let token = "axhub_pat_envfallback1234567890";

    let output = run_stdin(
        &["token-init", "--json"],
        "",
        &[("XDG_CONFIG_HOME", xdg_config_s), ("AXHUB_TOKEN", token)],
    );
    assert_eq!(output.status.code(), Some(0));
    assert_output_does_not_contain(&output, token);
    let json = stdout_json(&output);
    assert_eq!(json["stored"], true);
    assert_eq!(json["source"], "env:AXHUB_TOKEN");

    let token_path = xdg_config.join("axhub-plugin").join("token");
    assert_eq!(std::fs::read_to_string(&token_path).unwrap(), token);
    #[cfg(unix)]
    assert_eq!(
        std::fs::metadata(&token_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn cli_token_import_accepts_raw_and_json_without_leaking_token() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_config = temp.path().join("xdg-config");
    let xdg_config_s = xdg_config.to_str().unwrap();
    let token_path = xdg_config.join("axhub-plugin").join("token");

    let raw_token = "axhub_pat_rawimport1234567890";
    let raw = run_stdin(
        &["token-import", "--json"],
        raw_token,
        &[("XDG_CONFIG_HOME", xdg_config_s)],
    );
    assert_eq!(raw.status.code(), Some(0));
    assert_output_does_not_contain(&raw, raw_token);
    assert_eq!(std::fs::read_to_string(&token_path).unwrap(), raw_token);

    let json_token = "axhub_pat_jsonimport1234567890";
    let payload = serde_json::json!({"access_token": json_token, "token_type": "Bearer"});
    let json_import = run_stdin(
        &["token-import", "--json"],
        &payload.to_string(),
        &[("XDG_CONFIG_HOME", xdg_config_s)],
    );
    assert_eq!(json_import.status.code(), Some(0));
    assert_output_does_not_contain(&json_import, json_token);
    assert_eq!(std::fs::read_to_string(&token_path).unwrap(), json_token);
}

#[test]
fn cli_token_import_reports_invalid_payloads_without_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_config = temp.path().join("xdg-config");
    let xdg_config_s = xdg_config.to_str().unwrap();
    let token_path = xdg_config.join("axhub-plugin").join("token");

    let empty_json = run_stdin(
        &["token-import", "--json"],
        "   \n",
        &[("XDG_CONFIG_HOME", xdg_config_s)],
    );
    assert_eq!(empty_json.status.code(), Some(65));
    let json = stdout_json(&empty_json);
    assert_eq!(json["stored"], false);
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("access_token/token"));
    assert!(!token_path.exists());

    let short_token = "Bearer too-short";
    let short = run_stdin(
        &["token-import"],
        short_token,
        &[("XDG_CONFIG_HOME", xdg_config_s)],
    );
    assert_eq!(short.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&short.stderr).contains("access_token/token"));
    assert_output_does_not_contain(&short, "too-short");
    assert!(!token_path.exists());
}

#[test]
fn cli_token_commands_do_not_echo_token_like_unknown_options() {
    for command in ["token-init", "token-import"] {
        let mistaken_token_arg = "axhub_pat_mistakenarg1234567890";
        let output = Command::new(bin())
            .args([command, mistaken_token_arg])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(64), "{command}");
        assert_output_does_not_contain(&output, mistaken_token_arg);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unknown option"),
            "{command} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn cli_preauth_allows_axhub_help_flags_for_destructive_subcommands() {
    for command in [
        "axhub apps create --help",
        "axhub apps create -h",
        "axhub deploy create --help",
    ] {
        let payload = serde_json::json!({
            "session_id": "cli-e2e-session",
            "tool_call_id": format!("tc-help-{command}"),
            "tool_name": "Bash",
            "tool_input": {"command": command}
        })
        .to_string();
        let output = run_stdin(&["preauth-check"], &payload, &[]);
        assert_eq!(output.status.code(), Some(0), "{command}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("permissionDecision\":\"allow"),
            "{command}: {stdout}"
        );
    }
}

#[test]
fn cli_preauth_claims_pending_github_connect_consent() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let runtime = temp.path().join("run");
    std::fs::create_dir_all(&runtime).unwrap();
    let state_s = state.to_str().unwrap();
    let runtime_s = runtime.to_str().unwrap();
    let envs = [("XDG_STATE_HOME", state_s), ("XDG_RUNTIME_DIR", runtime_s)];

    let binding = serde_json::json!({
        "tool_call_id": "pending",
        "action": "github_connect",
        "app_id": "165",
        "profile": "",
        "branch": "main",
        "commit_sha": "",
        "context": {
            "repo": "realitsyourman/test2",
            "branch": "main",
            "account": "realitsyourman"
        }
    })
    .to_string();
    let mint = run_stdin(&["consent-mint"], &binding, &envs);
    assert_eq!(mint.status.code(), Some(0));

    let payload = serde_json::json!({
        "session_id": "cli-e2e-session",
        "tool_call_id": "tc-github-connect",
        "tool_name": "Bash",
        "tool_input": {
            "command": "axhub github connect 165 --repo realitsyourman/test2 --branch main --account realitsyourman --json"
        }
    })
    .to_string();
    let output = run_stdin(&["preauth-check"], &payload, &envs);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("permissionDecision\":\"allow"),
        "preauth stdout: {stdout}"
    );
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
    // Approach E (Phase 2): preflight-only context. cli_too_old 시 한국어 안내.
    assert!(stdout.contains("UserPromptSubmit"));
    assert!(stdout.contains("axhub 버전 확인 결과"));
    assert!(stdout.contains("너무 오래된 버전"));
    assert!(stdout.contains("axhub 업그레이드해줘"));
    // Approach E: skill path enforcement 폐기.
    assert!(
        !stdout.contains("skills/"),
        "no forced skill path: {stdout}"
    );
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

    // Approach E (Phase 2): Rust router does NOT classify intent. cmd_prompt_route emits
    // preflight-only context regardless of utterance. Skill matching happens via Claude
    // Code's native description matching against skills/*/SKILL.md frontmatter.
    let prompts = [
        "결제 앱 만들어줘",
        "Next.js 앱 만들어줘",
        "axhub.yaml 만들어줘",
        "환경변수 뭐 있어?",
        "환경 변수 확인",
        "회사 endpoint 바꿔",
        "profile current",
        "GitHub repo 연결해",
        "결과 봐",
        "axhub 뭐 새로 나왔어",
        "배포해",
        "내 axhub 앱 목록 보여줘",
        "앱 등록해",
        "nextjs-axhub 앱 지워",
        "이 앱 삭제해",
        "axhub 앱이 어떤 API 쓸 수 있는지 보여줘",
        "axhub 에 누구로 로그인돼있어",
        "로그 보여줘",
        "배포 상태 봐",
        "방금 거 되돌려",
        "axhub 새 버전 있어",
        "axhub 플러그인 업데이트",
        "axhub CLI 설치해줘",
        "axhub 좀 도와줘",
    ];

    for prompt in prompts {
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
        // No forced skill path enforcement (Approach E).
        assert!(
            !stdout.contains("skills/") && !stdout.contains("SKILL.md"),
            "no forced skill path for {prompt:?}: {stdout}"
        );
        // No "skill 워크플로우 적용" enforcement language.
        assert!(
            !stdout.contains("워크플로우를 우선 적용"),
            "no workflow enforcement for {prompt:?}: {stdout}"
        );
    }

    // Approach E: non-axhub prompts ("오늘 날씨", "rapid prototype") used to map to `{}`
    // because detect_prompt_route returned None. The new contract emits preflight context
    // regardless of intent (which is decided by Claude downstream). The new `no_intent_routing`
    // / `preflight_fail_soft` / `audit_fail_silent` tests below cover the contract.
}

// Approach E (Phase 2): no forced skill path enforcement, ever.
#[cfg(unix)]
#[test]
fn cli_prompt_route_no_forced_skills_context() {
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

    for prompt in [
        "배포해줘",
        "환경 점검",
        "어제 결제 페이지 띄워봐",
        "오늘 날씨",
    ] {
        let input =
            serde_json::json!({"hook_event_name":"UserPromptSubmit","prompt":prompt}).to_string();
        let output = run_stdin(
            &["prompt-route"],
            &input,
            &[("AXHUB_BIN", axhub.to_str().unwrap())],
        );
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("skills/"),
            "no skill path for {prompt:?}: {stdout}"
        );
        assert!(
            !stdout.contains("SKILL.md"),
            "no SKILL.md ref for {prompt:?}: {stdout}"
        );
        assert!(
            !stdout.contains("워크플로우를 우선 적용"),
            "no enforcement for {prompt:?}: {stdout}"
        );
    }
}

// Approach E (Phase 2): different prompts produce identical preflight-only output
// (no intent classification by Rust router).
#[cfg(unix)]
#[test]
fn cli_prompt_route_no_intent_routing() {
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

    let snapshot = |prompt: &str| -> String {
        let input =
            serde_json::json!({"hook_event_name":"UserPromptSubmit","prompt":prompt}).to_string();
        let output = run_stdin(
            &["prompt-route"],
            &input,
            &[("AXHUB_BIN", axhub.to_str().unwrap())],
        );
        assert_eq!(output.status.code(), Some(0));
        String::from_utf8_lossy(&output.stdout).into_owned()
    };

    // Different intents → identical hook output (Rust router is preflight-only).
    let a = snapshot("배포해줘");
    let b = snapshot("로그 봐");
    let c = snapshot("아무말 대잔치");
    assert_eq!(a, b);
    assert_eq!(b, c);
}

// Approach E (Phase 2): preflight failure must NOT block hook output.
#[cfg(unix)]
#[test]
fn cli_prompt_route_preflight_fail_soft() {
    // axhub binary missing → preflight cli_present=false, format_preflight_context still emits a line.
    let output = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"배포해줘"}"#,
        &[("AXHUB_BIN", "/no/such/axhub/binary/exists")],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "preflight failure must exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should produce non-empty additionalContext (cli_present=false branch) and never crash.
    // No skill path enforcement.
    assert!(
        !stdout.contains("skills/"),
        "no skill path on preflight fail: {stdout}"
    );
}

// Approach E (Phase 2): audit::append failures (e.g., AXHUB_NO_AUDIT=1) silently no-op.
#[cfg(unix)]
#[test]
fn cli_prompt_route_audit_fail_silent() {
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

    // AXHUB_NO_AUDIT=1 → audit::append no-ops. Hook still produces preflight context.
    let output = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"배포해줘"}"#,
        &[
            ("AXHUB_BIN", axhub.to_str().unwrap()),
            ("AXHUB_NO_AUDIT", "1"),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("UserPromptSubmit"));
    assert!(!stdout.contains("skills/"));
}

#[test]
fn cli_usage_preflight_resolve_list_and_session_start_paths_are_stable() {
    let no_args = run(&[]);
    assert_eq!(no_args.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&no_args.stderr).contains("Usage"));

    let unknown = run(&["unknown-subcommand"]);
    assert_eq!(unknown.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown subcommand"));

    let missing_path_kind = run(&["path"]);
    assert_eq!(missing_path_kind.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&missing_path_kind.stderr).contains("expected one of"));

    let unknown_path_kind = run(&["path", "unknown"]);
    assert_eq!(unknown_path_kind.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&unknown_path_kind.stderr).contains("unknown path kind"));

    let missing_app = run(&["list-deployments"]);
    assert_eq!(missing_app.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&missing_app.stderr).contains("--app-id"));

    for args in [
        ["list-deployments", "--app-id"].as_slice(),
        ["list-deployments", "--limit"].as_slice(),
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(64), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("requires a value"));
    }

    let invalid_app = Command::new(bin())
        .args(["list-deployments", "--app", "paydrop"])
        .env("AXHUB_TOKEN", "axhub_pat_abcdefghijklmnop")
        .env("AXHUB_ENDPOINT", "https://example.test")
        .output()
        .unwrap();
    assert_eq!(invalid_app.status.code(), Some(67));
    assert!(String::from_utf8_lossy(&invalid_app.stdout).contains("validation.app_id_invalid"));

    for (limit, expected) in [
        ("not-a-number", "invalid --limit"),
        ("0", "--limit must be between 1 and 100"),
        ("101", "--limit must be between 1 and 100"),
    ] {
        let invalid_limit = Command::new(bin())
            .args(["list-deployments", "--app-id", "42", "--limit", limit])
            .env("AXHUB_TOKEN", "axhub_pat_abcdefghijklmnop")
            .env("AXHUB_ENDPOINT", "https://example.test")
            .output()
            .unwrap();
        assert_eq!(invalid_limit.status.code(), Some(64), "limit={limit}");
        assert!(
            String::from_utf8_lossy(&invalid_limit.stderr).contains(expected),
            "limit={limit}; stderr={}",
            String::from_utf8_lossy(&invalid_limit.stderr)
        );
    }

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

    let statusline = run(&["statusline"]);
    assert_eq!(statusline.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&statusline.stdout).starts_with("axhub:"));

    let temp = tempfile::tempdir().unwrap();
    let xdg_config = temp.path().join("xdg-config");
    let token_file = Command::new(bin())
        .args(["path", "token-file"])
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .output()
        .unwrap();
    assert_eq!(token_file.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&token_file.stdout).trim(),
        xdg_config
            .join("axhub-plugin")
            .join("token")
            .to_string_lossy()
    );

    let user_profile = temp.path().join("Users").join("Vibe");
    let windows_home_token = Command::new(bin())
        .args(["path", "token-file"])
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("HOME")
        .env("USERPROFILE", &user_profile)
        .output()
        .unwrap();
    assert_eq!(windows_home_token.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&windows_home_token.stdout).trim(),
        user_profile
            .join(".config")
            .join("axhub-plugin")
            .join("token")
            .to_string_lossy()
    );

    let session = run(&["session-start"]);
    assert_eq!(session.status.code(), Some(0));
    let session_stdout = String::from_utf8_lossy(&session.stdout);
    assert!(session_stdout.contains("Rust runtime"));
    assert!(session_stdout.contains("AXHUB_NO_AUDIT"));
    assert!(session_stdout.contains("cleanup-audit --all"));
}

// Phase 4 routing-stats E2E helpers (XDG_STATE_HOME 격리).
//
// Each test writes a fake axhub binary to TempDir, points AXHUB_BIN at it, and
// scopes audit IO into TempDir/state via XDG_STATE_HOME. Hook input is the
// JSON envelope Claude Code sends (hook_event_name + prompt).

#[cfg(unix)]
fn fake_axhub(temp: &tempfile::TempDir) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.1.0 (commit fake, built fake, fake)"
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ] && [ "$3" = "--json" ]; then
  echo '{"user_email":"phase6@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["read","deploy"]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

#[cfg(unix)]
fn audit_dir_path(state: &std::path::Path) -> std::path::PathBuf {
    state.join("axhub-plugin")
}

#[cfg(unix)]
fn invoke_prompt_route(prompt: &str, axhub: &std::path::Path, state: &str) {
    let input =
        serde_json::json!({"hook_event_name":"UserPromptSubmit","prompt":prompt}).to_string();
    let output = run_stdin(
        &["prompt-route"],
        &input,
        &[
            ("AXHUB_BIN", axhub.to_str().unwrap()),
            ("XDG_STATE_HOME", state),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "prompt-route stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn cli_prompt_route_rotates_audit_on_write_path() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = audit_dir_path(&state);
    std::fs::create_dir_all(&dir).unwrap();

    let stale_date = (chrono::Utc::now() - chrono::Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let stale = dir.join(format!("routing-audit-{stale_date}.jsonl"));
    std::fs::write(&stale, "{}\n").unwrap();

    let output = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"오늘 날씨 알려줘"}"#,
        &[
            ("AXHUB_BIN", "/no/such/axhub/binary/exists"),
            ("XDG_STATE_HOME", state.display().to_string().as_str()),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(
        !stale.exists(),
        "prompt-route append should rotate stale audit files"
    );
}

#[cfg(unix)]
#[test]
fn cli_rotation_during_routing_stats_call() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = audit_dir_path(&state);
    std::fs::create_dir_all(&dir).unwrap();

    let stale_date = (chrono::Utc::now() - chrono::Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let stale = dir.join(format!("routing-audit-{stale_date}.jsonl"));
    let fresh = dir.join(format!("routing-audit-{today}.jsonl"));
    std::fs::write(&stale, "{}\n").unwrap();
    std::fs::write(&fresh, "{}\n").unwrap();

    let stats = run_stdin(
        &["routing-stats", "--json"],
        "",
        &[("XDG_STATE_HOME", state.display().to_string().as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));

    assert!(
        !stale.exists(),
        "stale audit file should be removed by rotate(7)"
    );
    assert!(fresh.exists(), "today's audit file should persist");
}

#[cfg(unix)]
#[test]
fn cli_cleanup_audit_all_yes_removes_audit_files() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = audit_dir_path(&state);
    std::fs::create_dir_all(&dir).unwrap();

    let stale_date = (chrono::Utc::now() - chrono::Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let stale = dir.join(format!("routing-audit-{stale_date}.jsonl"));
    let fresh = dir.join(format!("routing-audit-{today}.jsonl"));
    std::fs::write(&stale, "{}\n").unwrap();
    std::fs::write(&fresh, "{}\n").unwrap();

    let cleanup = run_stdin(
        &["cleanup-audit", "--all", "--yes"],
        "",
        &[("XDG_STATE_HOME", state.display().to_string().as_str())],
    );

    assert_eq!(cleanup.status.code(), Some(0));
    assert!(!stale.exists(), "stale audit file should be removed");
    assert!(
        !fresh.exists(),
        "fresh audit file should be removed by --all"
    );
}

#[cfg(unix)]
#[test]
fn cli_routing_stats_full_flow_korean_default() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    // Seed 3 prompt-route invocations.
    invoke_prompt_route("배포해줘", &axhub, &state_s);
    invoke_prompt_route("앱 목록 보여줘", &axhub, &state_s);
    invoke_prompt_route("이 코드 어떻게 동작해?", &axhub, &state_s);

    let stats = run_stdin(
        &["routing-stats", "--since", "7d"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&stats.stdout);
    assert!(stdout.contains("[지난 prompt 통계]"), "{stdout}");
    assert!(stdout.contains("총 prompt:"), "{stdout}");
    assert!(stdout.contains("audit log 위치:"), "{stdout}");
    assert!(stdout.contains("끄려면: AXHUB_NO_AUDIT=1"), "{stdout}");
    assert!(
        stdout.contains("삭제: axhub-helpers cleanup-audit --all"),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn cli_routing_stats_json_schema() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    invoke_prompt_route("배포해줘", &axhub, &state_s);

    let stats = run_stdin(
        &["routing-stats", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&stats.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    for key in [
        "total_prompts",
        "axhub_related",
        "axhub_related_rate",
        "auth_failed",
        "prompt_length_p50",
        "prompt_length_p95",
        "cli_versions",
        "top_axhub_hashes",
    ] {
        assert!(parsed.get(key).is_some(), "missing key: {key} in {stdout}");
    }
    assert!(parsed["top_axhub_hashes"].is_array());
    assert!(parsed["cli_versions"].is_object());
}

#[cfg(unix)]
#[test]
fn cli_routing_stats_top_n_filter() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    // Seed 5 unique axhub-related prompts.
    for prompt in [
        "배포해줘",
        "앱 만들어",
        "axhub 로그",
        "axhub status",
        "axhub auth",
    ] {
        invoke_prompt_route(prompt, &axhub, &state_s);
    }

    let stats = run_stdin(
        &["routing-stats", "--top", "2", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&stats.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    let top = parsed["top_axhub_hashes"].as_array().unwrap();
    assert!(
        top.len() <= 2,
        "--top 2 must cap at 2, got {} ({stdout})",
        top.len()
    );
}

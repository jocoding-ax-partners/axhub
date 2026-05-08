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

#[cfg(unix)]
#[test]
fn cli_prompt_route_rotates_audit_on_write_path() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = state.join("axhub-plugin");
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
            ("XDG_STATE_HOME", state.to_str().unwrap()),
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
fn cli_cleanup_audit_all_yes_removes_audit_files() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = state.join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let audit = dir.join("routing-audit-2000-01-01.jsonl");
    let unrelated = dir.join("notes.txt");
    std::fs::write(&audit, "{}\n").unwrap();
    std::fs::write(&unrelated, "{}\n").unwrap();

    let output = Command::new(bin())
        .args(["cleanup-audit", "--all", "--yes"])
        .env("XDG_STATE_HOME", state)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(!audit.exists());
    assert!(unrelated.exists());
}

#[cfg(unix)]
#[test]
fn cli_cleanup_audit_default_rotates_only_stale_files() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = state.join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let stale_date = (chrono::Utc::now() - chrono::Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let recent_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let stale = dir.join(format!("routing-audit-{stale_date}.jsonl"));
    let recent = dir.join(format!("routing-audit-{recent_date}.jsonl"));
    std::fs::write(&stale, "{}\n").unwrap();
    std::fs::write(&recent, "{}\n").unwrap();

    let output = Command::new(bin())
        .arg("cleanup-audit")
        .env("XDG_STATE_HOME", state)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("7일 이상 된 audit log 1 파일 삭제했어요")
    );
    assert!(!stale.exists());
    assert!(recent.exists());
}

#[cfg(unix)]
#[test]
fn cli_cleanup_audit_all_cancel_keeps_files_without_yes() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = state.join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    let audit = dir.join("routing-audit-2000-01-01.jsonl");
    std::fs::write(&audit, "{}\n").unwrap();

    let output = run_stdin(
        &["cleanup-audit", "--all"],
        "n\n",
        &[("XDG_STATE_HOME", state.to_str().unwrap())],
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("취소했어요."));
    assert!(audit.exists());
}

#[test]
fn cli_cleanup_audit_help_and_unknown_flags_are_stable() {
    let help = run(&["cleanup-audit", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("cleanup-audit"));

    let unknown = run(&["cleanup-audit", "--bogus"]);
    assert_eq!(unknown.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("알 수 없는 flag"));
}

#[test]
fn cli_consent_mint_rejects_binding_schema_drift_before_writing_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state").display().to_string();
    let runtime = temp.path().join("runtime").display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "schema-e2e-session"),
    ];
    let session_token = temp
        .path()
        .join("runtime/axhub/consent-schema-e2e-session.json");

    let unknown_action = serde_json::json!({
        "tool_call_id":"schema-e2e-session:tc-unknown",
        "action":"apps_publish",
        "app_id":"paydrop",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {}
    })
    .to_string();
    let rejected_unknown = run_stdin(&["consent-mint"], &unknown_action, &envs);
    assert_eq!(rejected_unknown.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&rejected_unknown.stderr).contains("binding_schema:unknown_action")
    );
    assert!(!session_token.exists());

    let missing_source = serde_json::json!({
        "tool_call_id":"schema-e2e-session:tc-apps-create",
        "action":"apps_create",
        "app_id":"",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"slug":"paydrop"}
    })
    .to_string();
    let rejected_missing_source = run_stdin(&["consent-mint"], &missing_source, &envs);
    assert_eq!(rejected_missing_source.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected_missing_source.stderr)
        .contains("binding_schema:missing_context:source"));
    assert!(!session_token.exists());

    let valid_apps_create = serde_json::json!({
        "tool_call_id":"schema-e2e-session:tc-apps-create",
        "action":"apps_create",
        "app_id":"",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"source":"apphub.yaml"}
    })
    .to_string();
    let minted = run_stdin(&["consent-mint"], &valid_apps_create, &envs);
    assert_eq!(minted.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&minted.stdout).contains("token_id"));
    assert!(session_token.exists());

    let interactive_binding = serde_json::json!({
        "tool_call_id":"pending",
        "action":"apps_create",
        "app_id":"",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"source":"interactive"}
    })
    .to_string();
    let pending_envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
    ];
    let interactive_minted = run_stdin(&["consent-mint"], &interactive_binding, &pending_envs);
    assert_eq!(interactive_minted.status.code(), Some(0));
    let interactive_allowed = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"schema-e2e-session","tool_call_id":"tc-interactive","tool_name":"Bash","tool_input":{"command":"axhub apps create --interactive --json"}}"#,
        &pending_envs,
    );
    assert_eq!(interactive_allowed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&interactive_allowed.stdout)
        .contains("permissionDecision\":\"allow"));
}

#[test]
fn cli_consent_mint_validate_only_has_no_runtime_or_key_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let runtime_dir = temp.path().join("runtime");
    let state = state_dir.display().to_string();
    let runtime = runtime_dir.display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "validate-only-session"),
    ];
    let binding = serde_json::json!({
        "tool_call_id":"validate-only-session:tc-validate",
        "action":"deploy_create",
        "app_id":"paydrop",
        "profile":"prod",
        "branch":"main",
        "commit_sha":"abc123",
        "context": {}
    })
    .to_string();

    let validated = run_stdin(&["consent-mint", "--validate-only"], &binding, &envs);
    assert_eq!(validated.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&validated.stdout);
    assert!(stdout.contains(r#""valid":true"#));
    assert!(stdout.contains(r#""action":"deploy_create"#));
    assert_no_consent_side_effects(&state_dir, &runtime_dir);
}

#[test]
fn cli_consent_mint_unknown_flags_fail_without_runtime_or_key_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let runtime_dir = temp.path().join("runtime");
    let state = state_dir.display().to_string();
    let runtime = runtime_dir.display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "unknown-flag-session"),
    ];
    let binding = serde_json::json!({
        "tool_call_id":"unknown-flag-session:tc-flag",
        "action":"deploy_create",
        "app_id":"paydrop",
        "profile":"prod",
        "branch":"main",
        "commit_sha":"abc123",
        "context": {}
    })
    .to_string();

    let rejected = run_stdin(&["consent-mint", "--unexpected"], &binding, &envs);
    assert_eq!(rejected.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("unknown option"));
    assert_no_consent_side_effects(&state_dir, &runtime_dir);
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

#[test]
fn cli_apps_delete_consent_binds_exact_command_target() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state").display().to_string();
    let runtime = temp.path().join("runtime").display().to_string();
    let pending_envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
    ];

    for (target, command, tool_id) in [
        (
            "nextjs-axhub",
            "axhub apps delete nextjs-axhub --yes --json",
            "toolu_app_delete_slug",
        ),
        (
            "165",
            "axhub apps delete 165 --yes --json",
            "toolu_app_delete_id",
        ),
    ] {
        let binding = serde_json::json!({
            "tool_call_id":"pending",
            "action":"apps_delete",
            "app_id":target,
            "profile":"",
            "branch":"",
            "commit_sha":"",
            "context": {"slug": target}
        })
        .to_string();
        let minted = run_stdin(&["consent-mint"], &binding, &pending_envs);
        assert_eq!(minted.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&minted.stdout).contains("consent-pending-"));

        let input = serde_json::json!({
            "session_id":"actual-claude-session",
            "tool_call_id":tool_id,
            "tool_name":"Bash",
            "tool_input":{"command": command}
        })
        .to_string();
        let allowed = run_stdin(&["preauth-check"], &input, &pending_envs);
        assert_eq!(allowed.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&allowed.stdout);
        assert!(stdout.contains("permissionDecision\":\"allow"), "{stdout}");
    }

    let session_envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "apps-delete-session"),
    ];
    let numeric_binding = serde_json::json!({
        "tool_call_id":"apps-delete-session:tc-app-delete",
        "action":"apps_delete",
        "app_id":"165",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"slug": "165"}
    })
    .to_string();
    let minted = run_stdin(&["consent-mint"], &numeric_binding, &session_envs);
    assert_eq!(minted.status.code(), Some(0));

    let slug_binding = serde_json::json!({
        "tool_call_id":"apps-delete-session:tc-app-delete",
        "action":"apps_delete",
        "app_id":"nextjs-axhub",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"slug": "nextjs-axhub"}
    })
    .to_string();
    let rejected = run_stdin(&["consent-verify"], &slug_binding, &session_envs);
    assert_eq!(rejected.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("binding_mismatch:app_id"));

    let context_mismatch = serde_json::json!({
        "tool_call_id":"apps-delete-session:tc-app-delete",
        "action":"apps_delete",
        "app_id":"165",
        "profile":"",
        "branch":"",
        "commit_sha":"",
        "context": {"slug": "nextjs-axhub"}
    })
    .to_string();
    let rejected = run_stdin(&["consent-verify"], &context_mismatch, &session_envs);
    assert_eq!(rejected.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("binding_mismatch:context"));
}

#[test]
fn cli_bootstrap_dry_run_does_not_create_state_or_gitignore() {
    let temp = tempfile::tempdir().unwrap();
    let output = run_in_dir(&["bootstrap", "--dry-run", "--json"], temp.path());
    assert_eq!(output.status.code(), Some(65));
    let json = stdout_json(&output);
    assert_eq!(json["state"], "template_required");
    assert_eq!(json["user_decision"], "template_required");
    assert!(!temp.path().join(".axhub").exists());
    assert!(!temp.path().join(".gitignore").exists());
}

#[test]
fn cli_bootstrap_auto_chain_plans_apps_create_without_hidden_remote_mutation() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path());
    let ledger = temp.path().join("axhub-call-ledger.txt");
    let output = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["state"], "consent_required_apps_create");
    assert_eq!(json["next_action"], "apps_create");
    assert_eq!(json["command"][0], "axhub");
    assert_eq!(json["command"][1], "apps");
    assert_eq!(json["command"][2], "create");
    assert_eq!(json["consent_binding"]["action"], "apps_create");
    assert_eq!(json["consent_binding"]["synthesized_by_helper"], true);
    assert!(json["binding_hash"].as_str().unwrap().len() >= 16);
    assert!(json["pending_action_id"]
        .as_str()
        .unwrap()
        .starts_with("apps_create:"));
    assert!(json["pending_action_hash"].as_str().unwrap().len() >= 16);
    assert!(
        !ledger.exists(),
        "bootstrap must not execute axhub internally"
    );
    assert!(temp.path().join(".axhub/bootstrap.state.json").exists());
    assert!(std::fs::read_to_string(temp.path().join(".gitignore"))
        .unwrap()
        .contains(".axhub/bootstrap.state.json"));

    let replayed = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(replayed.status.code(), Some(0));
    let replayed_json = stdout_json(&replayed);
    assert_eq!(
        replayed_json["pending_action_id"],
        json["pending_action_id"]
    );
    assert_eq!(
        replayed_json["pending_action_hash"],
        json["pending_action_hash"]
    );
    assert_eq!(replayed_json["binding_hash"], json["binding_hash"]);
    assert_eq!(replayed_json["command"], json["command"]);
    assert_eq!(replayed_json["consent_binding"], json["consent_binding"]);
    assert_eq!(
        replayed_json["retry_policy"],
        "no_retry_without_confirmed_idempotency"
    );
}

#[test]
fn cli_bootstrap_telemetry_markers_are_opt_in_redacted_and_re_entry_aware() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path());
    let state_home = temp.path().join("state-home");
    let state_home_str = state_home.display().to_string();
    let env_off = [
        ("AXHUB_TELEMETRY", "0"),
        ("XDG_STATE_HOME", state_home_str.as_str()),
        ("CLAUDE_SESSION_ID", "bootstrap_session_abc123"),
        ("AXHUB_PROFILE", "staging"),
    ];

    let off = run_in_dir_env(
        &["bootstrap", "--auto-chain", "--json"],
        temp.path(),
        &env_off,
    );
    assert_eq!(off.status.code(), Some(0));
    assert!(!state_home.join("axhub-plugin/usage.jsonl").exists());

    std::fs::remove_dir_all(temp.path().join(".axhub")).unwrap();
    std::fs::remove_file(temp.path().join(".gitignore")).unwrap();
    let env_on = [
        ("AXHUB_TELEMETRY", "1"),
        ("XDG_STATE_HOME", state_home_str.as_str()),
        ("CLAUDE_SESSION_ID", "bootstrap_session_abc123"),
        ("AXHUB_PROFILE", "staging"),
    ];

    let planned = run_in_dir_env(
        &["bootstrap", "--auto-chain", "--json"],
        temp.path(),
        &env_on,
    );
    assert_eq!(planned.status.code(), Some(0));
    let replayed = run_in_dir_env(
        &["bootstrap", "--auto-chain", "--json"],
        temp.path(),
        &env_on,
    );
    assert_eq!(replayed.status.code(), Some(0));

    let raw = std::fs::read_to_string(state_home.join("axhub-plugin/usage.jsonl")).unwrap();
    let events: Vec<serde_json::Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let event_names: Vec<&str> = events
        .iter()
        .filter_map(|event| event["event"].as_str())
        .collect();
    assert!(event_names.contains(&"bootstrap_phase_start"));
    assert!(event_names.contains(&"bootstrap_phase_end"));
    assert!(event_names.contains(&"bootstrap_re_entry_at_state"));
    assert!(event_names.contains(&"consent_synthesized_by_helper"));

    let allowed = [
        "ts",
        "session_id",
        "plugin_version",
        "cli_version",
        "helper_version",
        "event",
        "schema_version",
        "state",
        "phase",
        "outcome",
        "elapsed_ms",
        "decision_class",
        "retry_policy",
        "record_event",
    ];
    for event in &events {
        let obj = event.as_object().unwrap();
        for key in obj.keys() {
            assert!(
                allowed.contains(&key.as_str()),
                "unexpected telemetry key: {key}"
            );
        }
        let serialized = event.to_string();
        for forbidden in [
            "paydrop",
            "apphub.yaml",
            "axhub apps create",
            "Bearer ",
            "AXHUB_TOKEN",
            "https://",
            "stdout",
            "stderr",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "forbidden telemetry value {forbidden}: {serialized}"
            );
        }
        assert_eq!(event["schema_version"], "bootstrap-telemetry/v1");
    }

    let consent = events
        .iter()
        .find(|event| event["event"] == "consent_synthesized_by_helper")
        .unwrap();
    assert_eq!(consent["record_event"], "apps_create");
    assert_eq!(consent["decision_class"], "remote_destructive_plan");
    assert_eq!(
        consent["retry_policy"],
        "no_retry_without_confirmed_idempotency"
    );
}

#[test]
fn cli_bootstrap_record_rejects_duplicate_out_of_order_and_mismatched_pending_actions() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path());
    let planned = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(planned.status.code(), Some(0));
    let plan = stdout_json(&planned);
    let envelope = serde_json::json!({
        "schema_version": "bootstrap-record/v1",
        "pending_action_id": plan["pending_action_id"],
        "pending_action_hash": plan["pending_action_hash"],
        "command_argv": plan["command"],
        "exit_code": 0,
        "stdout_json": serde_json::from_str::<serde_json::Value>(include_str!("fixtures/bootstrap/apps_create.success.v1.json")).unwrap(),
        "stderr": ""
    });
    let recorded = run_stdin_in_dir(
        &["bootstrap", "--record", "apps_create", "--json"],
        &envelope.to_string(),
        temp.path(),
    );
    assert_eq!(recorded.status.code(), Some(0));
    assert_eq!(stdout_json(&recorded)["state"], "app_registered");

    let duplicate = run_stdin_in_dir(
        &["bootstrap", "--record", "apps_create", "--json"],
        &envelope.to_string(),
        temp.path(),
    );
    assert_eq!(duplicate.status.code(), Some(64));
    assert_eq!(
        stdout_json(&duplicate)["reason"],
        "record_duplicate_or_no_pending_action"
    );

    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path());
    let planned = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    let plan = stdout_json(&planned);
    let mut stale = envelope.clone();
    stale["pending_action_id"] = plan["pending_action_id"].clone();
    stale["pending_action_hash"] = serde_json::Value::String("bad-hash".into());
    stale["command_argv"] = plan["command"].clone();
    let mismatch = run_stdin_in_dir(
        &["bootstrap", "--record", "apps_create", "--json"],
        &stale.to_string(),
        temp.path(),
    );
    assert_eq!(mismatch.status.code(), Some(64));
    assert_eq!(
        stdout_json(&mismatch)["reason"],
        "record_pending_action_mismatch"
    );

    let out_of_order = run_stdin_in_dir(
        &["bootstrap", "--record", "deploy_create", "--json"],
        &serde_json::json!({
            "schema_version": "bootstrap-record/v1",
            "pending_action_id": plan["pending_action_id"],
            "pending_action_hash": plan["pending_action_hash"],
            "command_argv": plan["command"],
            "exit_code": 0,
            "stdout_json": serde_json::from_str::<serde_json::Value>(include_str!("fixtures/bootstrap/apps_create.success.v1.json")).unwrap(),
            "stderr": ""
        })
        .to_string(),
        temp.path(),
    );
    assert_eq!(out_of_order.status.code(), Some(64));
    assert_eq!(stdout_json(&out_of_order)["reason"], "record_out_of_order");
}

#[test]
fn cli_bootstrap_git_init_and_first_commit_are_user_decision_states() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path());
    let planned = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    let plan = stdout_json(&planned);
    let envelope = serde_json::json!({
        "schema_version": "bootstrap-record/v1",
        "pending_action_id": plan["pending_action_id"],
        "pending_action_hash": plan["pending_action_hash"],
        "command_argv": plan["command"],
        "exit_code": 0,
        "stdout_json": serde_json::from_str::<serde_json::Value>(include_str!("fixtures/bootstrap/apps_create.success.v1.json")).unwrap(),
        "stderr": ""
    });
    let recorded = run_stdin_in_dir(
        &["bootstrap", "--record", "apps_create", "--json"],
        &envelope.to_string(),
        temp.path(),
    );
    assert_eq!(recorded.status.code(), Some(0));

    let no_git = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(no_git.status.code(), Some(65));
    let no_git_json = stdout_json(&no_git);
    assert_eq!(no_git_json["state"], "git_init_required");
    assert_eq!(no_git_json["next_action"], "git_init");
    assert_eq!(no_git_json["command"], serde_json::json!(["git", "init"]));
    assert!(
        !temp.path().join(".git").exists(),
        "bootstrap must not run git init"
    );

    let git_init = Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(git_init.status.success());
    let first_commit = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(first_commit.status.code(), Some(65));
    let first_commit_json = stdout_json(&first_commit);
    assert_eq!(first_commit_json["state"], "first_commit_required");
    assert_eq!(first_commit_json["next_action"], "first_commit");
    assert_eq!(first_commit_json["command"][0], "git");
    assert_eq!(first_commit_json["command"][1], "commit");
}

#[test]
fn cli_bootstrap_record_validates_event_before_reading_stdin() {
    let temp = tempfile::tempdir().unwrap();
    let missing = run_in_dir(&["bootstrap", "--record", "--json"], temp.path());
    assert_eq!(missing.status.code(), Some(64));
    assert_eq!(stdout_json(&missing)["reason"], "record_event_missing");

    let unknown = run_in_dir(&["bootstrap", "--record", "unknown", "--json"], temp.path());
    assert_eq!(unknown.status.code(), Some(64));
    assert_eq!(stdout_json(&unknown)["reason"], "record_event_unknown");
}

#[test]
fn cli_bootstrap_malformed_deploy_success_records_terminal_stop_without_stale_pending() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path());
    let planned = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(planned.status.code(), Some(0));
    let plan = stdout_json(&planned);
    let recorded = record_apps_create_success(temp.path(), &plan);
    assert_eq!(recorded.status.code(), Some(0));

    init_git_with_commit(temp.path());
    let deploy_plan = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(deploy_plan.status.code(), Some(0));
    let deploy_plan_json = stdout_json(&deploy_plan);
    assert_eq!(deploy_plan_json["next_action"], "deploy_create");
    assert_eq!(
        deploy_plan_json["consent_binding"]["action"],
        "deploy_create"
    );

    let malformed = serde_json::json!({
        "schema_version": "bootstrap-record/v1",
        "pending_action_id": deploy_plan_json["pending_action_id"],
        "pending_action_hash": deploy_plan_json["pending_action_hash"],
        "command_argv": deploy_plan_json["command"],
        "exit_code": 0,
        "stdout_json": {},
        "stderr": ""
    });
    let malformed_record = run_stdin_in_dir(
        &["bootstrap", "--record", "deploy_create", "--json"],
        &malformed.to_string(),
        temp.path(),
    );
    assert_eq!(malformed_record.status.code(), Some(65));
    let malformed_json = stdout_json(&malformed_record);
    assert_eq!(malformed_json["state"], "backend_contract_missing_defaults");
    assert_eq!(
        malformed_json["reason"],
        "deploy_create_missing_deployment_id"
    );

    let replay = run_in_dir(&["bootstrap", "--auto-chain", "--json"], temp.path());
    assert_eq!(replay.status.code(), Some(65));
    let replay_json = stdout_json(&replay);
    assert_eq!(replay_json["state"], "backend_contract_missing_defaults");
    assert!(replay_json.get("pending_action_id").is_none());
    assert!(replay_json.get("command").is_none());

    let state_raw =
        std::fs::read_to_string(temp.path().join(".axhub/bootstrap.state.json")).unwrap();
    let state_json: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
    assert!(state_json.get("pending_action").is_none());
    assert_eq!(state_json["completed_actions"].as_array().unwrap().len(), 2);
}

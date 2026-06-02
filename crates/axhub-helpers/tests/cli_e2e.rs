use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

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

/// Like `run_stdin`, but can also *remove* inherited env vars (e.g.
/// `XDG_RUNTIME_DIR`). `run_stdin` only adds vars, so it cannot reproduce the
/// macOS Claude Code condition where `XDG_RUNTIME_DIR` is unset and the consent
/// `runtime_root()` falls back to a per-process `$TMPDIR`.
fn run_stdin_with_env(
    args: &[&str],
    stdin: &str,
    envs: &[(&str, &str)],
    remove_envs: &[&str],
) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        command.env(k, v);
    }
    for k in remove_envs {
        command.env_remove(k);
    }
    let mut child = command.spawn().unwrap();
    write_stdin_allowing_early_exit(child.stdin.as_mut().unwrap(), stdin);
    child.wait_with_output().unwrap()
}

fn assert_no_consent_side_effects(state_dir: &Path, runtime_dir: &Path) {
    assert!(!state_dir.exists());
    assert!(!runtime_dir.exists());
}

fn write_private_test_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn write_expired_session_consent(
    state: &Path,
    runtime: &Path,
    session_id: &str,
    tool_call_id: &str,
) {
    let key = [11u8; 32];
    write_private_test_file(&state.join("axhub").join("hmac-key"), &key);

    let now = chrono::Utc::now().timestamp();
    let claims = serde_json::json!({
        "tool_call_id": format!("{session_id}:{tool_call_id}"),
        "action": "auth_login",
        "app_id": "_",
        "profile": "default",
        "branch": "_",
        "commit_sha": "_",
        "context": {},
        "synthesized_by_helper": false,
        "jti": "expired-test",
        "iat": now - 120,
        "exp": now - 60
    });
    let jwt = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&key),
    )
    .unwrap();
    let body = serde_json::json!({
        "token_id": "expired-test",
        "jwt": jwt,
        "expires_at": "expired",
        "session_id": session_id
    });
    write_private_test_file(
        &runtime
            .join("axhub")
            .join(format!("consent-{session_id}.json")),
        serde_json::to_string(&body).unwrap().as_bytes(),
    );
}

fn write_expired_pending_consent(state: &Path, runtime: &Path) {
    let key = [13u8; 32];
    write_private_test_file(&state.join("axhub").join("hmac-key"), &key);

    let now = chrono::Utc::now().timestamp();
    let claims = serde_json::json!({
        "tool_call_id": "pending",
        "action": "auth_login",
        "app_id": "_",
        "profile": "default",
        "branch": "_",
        "commit_sha": "_",
        "context": {},
        "synthesized_by_helper": false,
        "jti": "expired-pending-test",
        "iat": now - 120,
        "exp": now - 60
    });
    let jwt = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&key),
    )
    .unwrap();
    let body = serde_json::json!({
        "token_id": "expired-pending-test",
        "jwt": jwt,
        "expires_at": "expired",
        "session_id": "pending"
    });
    write_private_test_file(
        &runtime
            .join("axhub")
            .join("consent-pending-expired-test.json"),
        serde_json::to_string(&body).unwrap().as_bytes(),
    );
}

#[test]
fn cli_diagnose_hitl_rejects_non_tty_stdin_without_empty_capture_success() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let runtime = temp.path().join("runtime");
    let prompts = temp.path().join("prompts.json");
    let output = temp.path().join("captured.json");
    std::fs::write(
        &prompts,
        r#"[{"kind":"capture","key":"failure_note","message":"paste failure output"}]"#,
    )
    .unwrap();

    let run = Command::new(bin())
        .args([
            "diagnose",
            "hitl",
            "--session",
            "loop-non-tty",
            "--prompts",
            prompts.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .env("XDG_STATE_HOME", &state)
        .env("XDG_RUNTIME_DIR", &runtime)
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert_eq!(
        run.status.code(),
        Some(65),
        "non-interactive HITL must fail closed instead of storing an empty capture"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("TTY unavailable"),
        "stderr should explain the interactive requirement, got: {stderr}"
    );
    assert!(
        !output.exists(),
        "non-TTY fallback must not persist an empty captured.json"
    );
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

fn run_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().unwrap()
}

#[cfg(unix)]
fn fake_verify_axhub(
    temp: &tempfile::TempDir,
    name: &str,
    status_stdout: &str,
    status_exit: i32,
    logs_stdout: &str,
    logs_exit: i32,
) -> std::path::PathBuf {
    let axhub = temp.path().join(name);
    std::fs::write(
        &axhub,
        format!(
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.15.3"
  exit 0
fi
if [ "$1 $2 $3" = "--json deploy list" ]; then
  echo '{{"items":[{{"id":"dep-fixture","status":"running","started_at":"2026-05-11T00:00:00Z"}}]}}'
  exit 0
fi
if [ "$1 $2 $3" = "--json deploy status" ]; then
  cat <<'AXHUB_STATUS'
{status_stdout}
AXHUB_STATUS
  exit {status_exit}
fi
if [ "$1 $2 $3" = "--json deploy logs" ]; then
  cat <<'AXHUB_LOGS'
{logs_stdout}
AXHUB_LOGS
  exit {logs_exit}
fi
exit 64
"#,
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

#[cfg(unix)]
fn fake_slow_status_axhub(temp: &tempfile::TempDir) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-slow-status");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.15.3"
  exit 0
fi
if [ "$1 $2 $3" = "--json deploy list" ]; then
  echo '{"items":[{"id":"dep-slow","status":"running","started_at":"2026-05-11T00:00:00Z"}]}'
  exit 0
fi
if [ "$1 $2 $3" = "--json deploy status" ]; then
  sleep 30
  exit 0
fi
if [ "$1 $2 $3" = "--json deploy logs" ]; then
  echo "INFO still serving"
  exit 0
fi
exit 64
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

#[cfg(unix)]
fn fake_list_deployments_axhub(
    temp: &tempfile::TempDir,
    stdout: &str,
    exit_code: i32,
) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-list-deployments");
    std::fs::write(
        &axhub,
        format!(
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.15.3"
  exit 0
fi
if [ "$1 $2 $3" = "--json deploy list" ]; then
  cat <<'AXHUB_DEPLOYS'
{stdout}
AXHUB_DEPLOYS
  exit {exit_code}
fi
exit 64
"#,
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

#[cfg(windows)]
fn fake_list_deployments_axhub(
    temp: &tempfile::TempDir,
    stdout: &str,
    exit_code: i32,
) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-list-deployments.cmd");
    std::fs::write(
        &axhub,
        format!(
            r#"@echo off
if "%~1"=="--version" (
  echo axhub 0.15.3
  exit /b 0
)
if "%~1 %~2 %~3"=="--json deploy list" (
  echo {stdout}
  exit /b {exit_code}
)
exit /b 64
"#,
        ),
    )
    .unwrap();
    axhub
}

#[cfg(unix)]
fn fake_deploy_prep_axhub(temp: &tempfile::TempDir, version_stdout: &str) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-deploy-prep");
    std::fs::write(
        &axhub,
        format!(
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  cat <<'AXHUB_VERSION'
{version_stdout}
AXHUB_VERSION
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  cat <<'AXHUB_AUTH'
{{"user_email":"dev@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}}
AXHUB_AUTH
  exit 0
fi
if [ "$1" = "apps" ] && [ "$2" = "list" ]; then
  cat <<'AXHUB_APPS'
[{{"id":42,"slug":"paydrop"}}]
AXHUB_APPS
  exit 0
fi
exit 0
"#,
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
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
        .args(["add", "axhub.yaml", ".gitignore"])
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
        dir.join("axhub.yaml"),
        "name: Paydrop\nslug: paydrop\nframework: nextjs\n",
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn cli_verify_requires_app_id() {
    let output = run_env(&["verify", "--json"], &[]);
    assert_eq!(output.status.code(), Some(64));
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Error must mention both the canonical --app and legacy --app-id alias
    // so users following either skill example get the same hint.
    assert!(
        stderr.contains("--app"),
        "stderr should mention --app: {stderr}"
    );
    assert!(
        stderr.contains("--app-id"),
        "stderr should keep --app-id alias hint: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn cli_verify_json_live_uses_status_and_logs_probes() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_verify_axhub(
        &temp,
        "axhub-live",
        r#"{"state":"live","last_deploy_id":"dep-live","last_deploy_age_secs":42}"#,
        0,
        "INFO boot\nINFO ready\n",
        0,
    );

    let output = run_env(
        &["verify", "--json", "--app-id=paydrop"],
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["verdict"], "live");
    assert_eq!(json["state"], "live");
    assert_eq!(json["last_deploy_id"], "dep-live");
    assert_eq!(json["last_deploy_age_secs"], 42);
    assert!(json["errors"].as_array().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn cli_verify_plain_suspect_humanizes_runtime_errors() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_verify_axhub(
        &temp,
        "axhub-suspect",
        r#"{"state":"live","last_deploy_id":"dep-sus","last_deploy_age_secs":30}"#,
        0,
        "ERROR connection refused\nINFO retry\n",
        0,
    );

    let output = run_env(
        &["verify", "--app-id", "paydrop"],
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠️ 의심"), "{stdout}");
    assert!(stdout.contains("runtime 에러 1건"), "{stdout}");
    assert!(stdout.contains("다시 확인해줘"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn cli_verify_json_not_live_exits_usage_code() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_verify_axhub(
        &temp,
        "axhub-not-live",
        r#"{"state":"rolled_back","last_deploy_id":"dep-old","last_deploy_age_secs":900}"#,
        0,
        "INFO old deploy\n",
        0,
    );

    let output = run_env(
        &["verify", "--json", "--app-id", "paydrop"],
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(64));
    let json = stdout_json(&output);
    assert_eq!(json["verdict"], "not_live");
    assert_eq!(json["state"], "rolled_back");
    assert!(
        json["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().unwrap().contains("rolled_back")),
        "{json}",
    );
}

#[cfg(unix)]
#[test]
fn cli_verify_times_out_slow_status_probe() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_slow_status_axhub(&temp);

    let started = std::time::Instant::now();
    let output = run_env(
        &["verify", "--json", "--app-id", "paydrop"],
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );

    assert!(
        started.elapsed() < std::time::Duration::from_secs(12),
        "verify should enforce the 5s status probe timeout"
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["verdict"], "suspect");
    assert!(
        json["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().unwrap().contains("status timeout")),
        "{json}",
    );
}

#[cfg(unix)]
#[test]
fn cli_deploy_prep_quality_gate_passes_in_json() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_deploy_prep_axhub(&temp, "axhub 0.15.3");

    let output = run_env(
        &[
            "deploy-prep",
            "--intent",
            "deploy",
            "--user-utterance",
            "paydrop",
            "--json",
        ],
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );

    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["quality_gate"]["passed"], true);
    assert!(json["quality_gate"]["violations"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(json["exit_code"], 0);
}

#[cfg(unix)]
#[test]
fn cli_deploy_prep_quality_gate_fails_closed_with_sub_key() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_deploy_prep_axhub(&temp, "axhub development-build");

    let output = run_env(
        &[
            "deploy-prep",
            "--intent",
            "deploy",
            "--user-utterance",
            "paydrop",
            "--json",
        ],
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );

    assert_eq!(output.status.code(), Some(64));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("axhub-error-sub-key: 64:validation.quality_gate_failed"),
        "{stderr}"
    );
    let json = stdout_json(&output);
    assert_eq!(json["quality_gate"]["passed"], false);
    assert_eq!(json["exit_code"], 64);
    assert!(
        json["quality_gate"]["violations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|violation| violation["kind"] == "missing_cli_version"),
        "{json}",
    );
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
fn cli_config_get_set_json_plaintext_and_opt_out_are_stable() {
    let temp = tempfile::tempdir().unwrap();
    let config_home = temp.path().join("xdg-config");
    let home = temp.path().join("home");
    let config_home_s = config_home.to_string_lossy().to_string();
    let home_s = home.to_string_lossy().to_string();

    let missing = run_env(
        &["config", "get", "ignore_too_new_until", "--json"],
        &[("XDG_CONFIG_HOME", &config_home_s), ("HOME", &home_s)],
    );
    assert_eq!(missing.status.code(), Some(0));
    let missing_json = stdout_json(&missing);
    assert_eq!(missing_json["key"], "ignore_too_new_until");
    assert_eq!(missing_json["value"], serde_json::Value::Null);

    let set = run_env(
        &["config", "set", "ignore_too_new_until", "v0.12.5"],
        &[("XDG_CONFIG_HOME", &config_home_s), ("HOME", &home_s)],
    );
    assert_eq!(
        set.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&set.stderr)
    );

    let plain = run_env(
        &["config", "get", "ignore_too_new_until"],
        &[("XDG_CONFIG_HOME", &config_home_s), ("HOME", &home_s)],
    );
    assert_eq!(plain.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&plain.stdout).trim(), "v0.12.5");

    let disabled_json = run_env(
        &["config", "get", "ignore_too_new_until", "--json"],
        &[
            ("XDG_CONFIG_HOME", &config_home_s),
            ("HOME", &home_s),
            ("AXHUB_CLI_TOO_NEW_DISMISS", "0"),
        ],
    );
    assert_eq!(disabled_json.status.code(), Some(0));
    assert_eq!(
        stdout_json(&disabled_json)["value"],
        serde_json::Value::Null
    );

    let disabled_plain = run_env(
        &["config", "get", "ignore_too_new_until"],
        &[
            ("XDG_CONFIG_HOME", &config_home_s),
            ("HOME", &home_s),
            ("AXHUB_CLI_TOO_NEW_DISMISS", "0"),
        ],
    );
    assert_eq!(disabled_plain.status.code(), Some(1));
}

#[test]
fn cli_config_usage_errors_are_stable() {
    let temp = tempfile::tempdir().unwrap();
    let config_home_s = temp.path().join("xdg-config").to_string_lossy().to_string();
    let home_s = temp.path().join("home").to_string_lossy().to_string();
    let envs = [
        ("XDG_CONFIG_HOME", config_home_s.as_str()),
        ("HOME", home_s.as_str()),
    ];

    for args in [
        &["config"][..],
        &["config", "get"][..],
        &["config", "set"][..],
        &["config", "set", "ignore_too_new_until"][..],
        &["config", "unknown"][..],
        &["config", "set", "not_a_real_key", "x"][..],
    ] {
        let output = run_env(args, &envs);
        assert_eq!(
            output.status.code(),
            Some(64),
            "args={args:?} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn cli_auth_refresh_bg_opt_out_and_missing_cli_are_non_blocking() {
    let temp = tempfile::tempdir().unwrap();
    let home_s = temp.path().join("home").to_string_lossy().to_string();
    let missing_bin = temp
        .path()
        .join("missing-axhub")
        .to_string_lossy()
        .to_string();

    let disabled = run_env(
        &["auth-refresh-bg"],
        &[
            ("HOME", &home_s),
            ("AXHUB_BIN", &missing_bin),
            ("AXHUB_AUTH_BG_REFRESH", "0"),
        ],
    );
    assert_eq!(disabled.status.code(), Some(0));
    assert!(!temp
        .path()
        .join("home/.config/axhub-plugin/auth-refresh-status.json")
        .exists());

    let missing = run_env(
        &["auth-refresh-bg"],
        &[("HOME", &home_s), ("AXHUB_BIN", &missing_bin)],
    );
    assert_eq!(missing.status.code(), Some(0));
    let sentinel = std::fs::read_to_string(
        temp.path()
            .join("home/.config/axhub-plugin/auth-refresh-status.json"),
    )
    .unwrap();
    assert!(sentinel.contains("\"success\":false"));
    assert!(sentinel.contains("\"status\":\"axhub_cli_missing\""));
}

#[cfg(unix)]
#[test]
fn cli_auth_refresh_bg_records_success_and_failure_sentinels() {
    fn fake_axhub(dir: &Path, name: &str, login_exit: i32) -> String {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!(
                "#!/usr/bin/env sh\nif [ \"$1\" = \"--version\" ]; then echo 'axhub 0.16.0'; exit 0; fi\nif [ \"$1 $2 $3 $4\" = \"--json auth refresh --no-browser\" ]; then exit {login_exit}; fi\nexit 64\n"
            ),
        )
        .unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path.to_string_lossy().to_string()
    }

    let temp = tempfile::tempdir().unwrap();
    let success_home = temp.path().join("success-home");
    let success_home_s = success_home.to_string_lossy().to_string();
    let success_bin = fake_axhub(temp.path(), "axhub-success", 0);
    let success = run_env(
        &["auth-refresh-bg"],
        &[("HOME", &success_home_s), ("AXHUB_BIN", &success_bin)],
    );
    assert_eq!(
        success.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&success.stderr)
    );
    let success_sentinel =
        std::fs::read_to_string(success_home.join(".config/axhub-plugin/auth-refresh-status.json"))
            .unwrap();
    assert!(success_sentinel.contains("\"success\":true"));
    assert!(success_sentinel.contains("\"status\":\"ok\""));

    let fail_home = temp.path().join("fail-home");
    let fail_home_s = fail_home.to_string_lossy().to_string();
    let fail_bin = fake_axhub(temp.path(), "axhub-fail", 7);
    let failure = run_env(
        &["auth-refresh-bg"],
        &[("HOME", &fail_home_s), ("AXHUB_BIN", &fail_bin)],
    );
    assert_eq!(failure.status.code(), Some(1));
    let failure_sentinel =
        std::fs::read_to_string(fail_home.join(".config/axhub-plugin/auth-refresh-status.json"))
            .unwrap();
    assert!(failure_sentinel.contains("\"success\":false"));
    assert!(failure_sentinel.contains("\"status\":\"fail\""));
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
fn cli_legacy_passthrough_rejects_unknown_args_without_secret_echo() {
    let mistaken_token_arg = "axhub_pat_ultraqa_mistakenarg1234567890";
    for args in [
        ["preflight", "--bogus", mistaken_token_arg].as_slice(),
        ["resolve", "--bogus", mistaken_token_arg].as_slice(),
        ["routing-dashboard", "--bogus", mistaken_token_arg].as_slice(),
        ["mark", "--bogus", mistaken_token_arg].as_slice(),
        ["emit-deploy-complete", "--bogus", mistaken_token_arg].as_slice(),
        ["deploy-prep", "--bogus", mistaken_token_arg].as_slice(),
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(64), "{args:?}");
        assert_output_does_not_contain(&output, mistaken_token_arg);
    }
}

#[test]
fn cli_legacy_json_compat_flags_remain_accepted() {
    let preflight = Command::new(bin())
        .args(["preflight", "--json"])
        .env("AXHUB_BIN", "/definitely-not-axhub")
        .output()
        .unwrap();
    assert_eq!(preflight.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&preflight.stdout).contains(r#""cli_present":false"#));
    assert!(
        !String::from_utf8_lossy(&preflight.stderr).contains("unknown option"),
        "preflight --json must stay as a no-op compatibility flag"
    );

    let resolve = Command::new(bin())
        .args(["resolve", "--user-utterance", "paydrop", "--json"])
        .env("AXHUB_BIN", "/definitely-not-axhub")
        .output()
        .unwrap();
    assert_eq!(resolve.status.code(), Some(65));
    assert!(String::from_utf8_lossy(&resolve.stdout).contains("auth_parse_error"));
    assert!(
        !String::from_utf8_lossy(&resolve.stderr).contains("unknown option"),
        "resolve --json must stay as a no-op compatibility flag"
    );
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

/// FR-001/FR-002 regression: a pending consent minted in one `$TMPDIR` MUST be
/// found by a `preauth-check` hook spawned with a DIFFERENT `$TMPDIR` when
/// `XDG_RUNTIME_DIR` is unset (the macOS Claude Code condition). Before the
/// `runtime_root()` fallback fix this DENYs (mint dir != read dir); after, it
/// ALLOWs. Existing consent tests all set `XDG_RUNTIME_DIR`, which masked this.
#[cfg(unix)]
#[test]
fn cli_preauth_allows_when_tmpdir_differs_and_xdg_runtime_unset() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let tmp_a = temp.path().join("tmp-a");
    let tmp_b = temp.path().join("tmp-b");
    std::fs::create_dir_all(&tmp_a).unwrap();
    std::fs::create_dir_all(&tmp_b).unwrap();
    let state_s = state.to_str().unwrap();
    let tmp_a_s = tmp_a.to_str().unwrap();
    let tmp_b_s = tmp_b.to_str().unwrap();

    // mint a pending auth_login consent with TMPDIR=A, XDG_RUNTIME_DIR removed.
    let binding = serde_json::json!({
        "tool_call_id": "pending",
        "action": "auth_login",
        "app_id": "_",
        "profile": "default",
        "branch": "_",
        "commit_sha": "_",
        "context": {}
    })
    .to_string();
    let mint = run_stdin_with_env(
        &["consent-mint"],
        &binding,
        &[("XDG_STATE_HOME", state_s), ("TMPDIR", tmp_a_s)],
        &["XDG_RUNTIME_DIR"],
    );
    assert_eq!(
        mint.status.code(),
        Some(0),
        "mint stderr: {}",
        String::from_utf8_lossy(&mint.stderr)
    );

    // preauth-check with a DIFFERENT TMPDIR=B (hook subprocess simulation).
    let payload = r#"{"session_id":"actual-claude-session","tool_call_id":"toolu_auth","tool_name":"Bash","tool_input":{"command":"axhub auth login"}}"#;
    let out = run_stdin_with_env(
        &["preauth-check"],
        payload,
        &[("XDG_STATE_HOME", state_s), ("TMPDIR", tmp_b_s)],
        &["XDG_RUNTIME_DIR"],
    );
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("permissionDecision\":\"allow"),
        "differing TMPDIR must still resolve consent; preauth stdout: {stdout}"
    );

    // FR-003 single-use: the pending consent is consumed (file removed) after a
    // successful claim. With XDG_RUNTIME_DIR unset the fallback dir is
    // <XDG_STATE_HOME>/axhub/runtime.
    let runtime_dir = state.join("axhub").join("runtime");
    let leftover_pending = std::fs::read_dir(&runtime_dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("consent-pending-")
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(
        leftover_pending, 0,
        "pending consent must be consumed (single-use) after a successful claim"
    );
}

/// FR-004: the no-XDG fallback consent dir (`<XDG_STATE_HOME>/axhub/runtime`) is
/// created `0700` and consent files `0600`, so other users on a shared host
/// can't read or hijack a consent token.
#[cfg(unix)]
#[test]
fn cli_consent_fallback_dir_and_file_are_private() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.to_str().unwrap();
    let binding = serde_json::json!({
        "tool_call_id": "pending",
        "action": "auth_login",
        "app_id": "_",
        "profile": "default",
        "branch": "_",
        "commit_sha": "_",
        "context": {}
    })
    .to_string();
    let mint = run_stdin_with_env(
        &["consent-mint"],
        &binding,
        &[("XDG_STATE_HOME", state_s)],
        &["XDG_RUNTIME_DIR"],
    );
    assert_eq!(mint.status.code(), Some(0));

    let runtime_dir = state.join("axhub").join("runtime");
    let dir_mode = std::fs::metadata(&runtime_dir)
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        dir_mode, 0o700,
        "fallback runtime dir must be 0700, got {dir_mode:o}"
    );

    let pending: Vec<_> = std::fs::read_dir(&runtime_dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("consent-pending-")
        })
        .collect();
    assert_eq!(pending.len(), 1, "exactly one pending consent expected");
    let file_mode = pending[0].metadata().unwrap().permissions().mode() & 0o777;
    assert_eq!(
        file_mode, 0o600,
        "consent file must be 0600, got {file_mode:o}"
    );
}

#[test]
fn cli_consent_fallback_ignores_empty_xdg_state_home() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let tmp = temp.path().join("tmp");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&tmp).unwrap();
    let binding = serde_json::json!({
        "tool_call_id": "pending",
        "action": "auth_login",
        "app_id": "_",
        "profile": "default",
        "branch": "_",
        "commit_sha": "_",
        "context": {}
    })
    .to_string();
    let mint = run_stdin_with_env(
        &["consent-mint"],
        &binding,
        &[
            ("XDG_STATE_HOME", ""),
            ("HOME", home.to_str().unwrap()),
            ("TMPDIR", tmp.to_str().unwrap()),
        ],
        &["XDG_RUNTIME_DIR"],
    );
    assert_eq!(
        mint.status.code(),
        Some(0),
        "mint stderr: {}",
        String::from_utf8_lossy(&mint.stderr)
    );

    let runtime_dir = home
        .join(".local")
        .join("state")
        .join("axhub")
        .join("runtime");
    assert!(
        runtime_dir.exists(),
        "empty XDG_STATE_HOME must fall back to HOME, not cwd or TMPDIR"
    );
}

/// FR-006 / contract C4: a deny carries the Korean reason in BOTH the canonical
/// `permissionDecisionReason` field (the one Claude Code's deny UI surfaces) and
/// the `systemMessage` prose channel, and still exits 0 (fail-open).
#[test]
fn cli_preauth_deny_surfaces_reason_in_both_channels() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let runtime = temp.path().join("run");
    std::fs::create_dir_all(&runtime).unwrap();
    let envs = [
        ("XDG_STATE_HOME", state.to_str().unwrap()),
        ("XDG_RUNTIME_DIR", runtime.to_str().unwrap()),
    ];
    // auth login with no consent minted → deny.
    let out = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"s","tool_call_id":"t","tool_name":"Bash","tool_input":{"command":"axhub auth login"}}"#,
        &envs,
    );
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("permissionDecision\":\"deny"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("permissionDecisionReason"),
        "deny must carry the canonical reason field: {stdout}"
    );
    assert!(
        stdout.contains("systemMessage"),
        "deny must keep the systemMessage channel: {stdout}"
    );
    assert!(
        stdout.contains("사전 승인이 필요해요"),
        "reason must be the Korean hint: {stdout}"
    );
}

#[test]
fn cli_preauth_deny_surfaces_expired_consent_reason() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let runtime = temp.path().join("run");
    let session_id = "expired-session";
    let tool_call_id = "tool-expired";
    write_expired_session_consent(&state, &runtime, session_id, tool_call_id);

    let payload = format!(
        r#"{{"session_id":"{session_id}","tool_call_id":"{tool_call_id}","tool_name":"Bash","tool_input":{{"command":"axhub auth login"}}}}"#
    );
    let out = run_stdin_with_env(
        &["preauth-check"],
        &payload,
        &[
            ("XDG_STATE_HOME", state.to_str().unwrap()),
            ("XDG_RUNTIME_DIR", runtime.to_str().unwrap()),
        ],
        &["CLAUDE_SESSION_ID"],
    );
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("permissionDecision\":\"deny"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("permissionDecisionReason"),
        "deny must carry the canonical reason field: {stdout}"
    );
    assert!(
        stdout.contains("사전 승인이 만료됐어요"),
        "expired consent must surface the TTL-specific reason: {stdout}"
    );
}

#[test]
fn cli_preauth_deny_surfaces_expired_pending_consent_reason() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let runtime = temp.path().join("run");
    write_expired_pending_consent(&state, &runtime);

    let out = run_stdin_with_env(
        &["preauth-check"],
        r#"{"session_id":"actual-session","tool_call_id":"tool-auth","tool_name":"Bash","tool_input":{"command":"axhub auth login"}}"#,
        &[
            ("XDG_STATE_HOME", state.to_str().unwrap()),
            ("XDG_RUNTIME_DIR", runtime.to_str().unwrap()),
        ],
        &["CLAUDE_SESSION_ID"],
    );
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("permissionDecision\":\"deny"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("사전 승인이 만료됐어요"),
        "expired pending consent must surface the TTL-specific reason: {stdout}"
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
    // Phase 26: prompt-route emits tagged English additionalContext only.
    assert!(stdout.contains("UserPromptSubmit"));
    assert!(stdout.contains("<axhub-preflight-status>"));
    assert!(stdout.contains("below required range"));
    assert!(stdout.contains("run `axhub update` before axhub commands."));
    assert!(stdout.contains("Skip: AXHUB_DISABLE_HOOK=prompt-route"));
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
  echo "axhub 0.15.3 (commit fake, built fake, fake)"
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
  echo "axhub 0.15.3 (commit fake, built fake, fake)"
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
  echo "axhub 0.15.3 (commit fake, built fake, fake)"
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
  echo "axhub 0.15.3 (commit fake, built fake, fake)"
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
    assert!(String::from_utf8_lossy(&missing_app.stderr).contains("--app"));

    for args in [
        ["list-deployments", "--app-id"].as_slice(),
        ["list-deployments", "--limit"].as_slice(),
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(64), "{args:?}");
        // clap 마이그레이션: value 누락 usage-error 문구가 generic "unknown option" 으로
        // (FR-006 dev-facing usage-error wording). exit 64 계약은 보존.
        assert!(String::from_utf8_lossy(&output.stderr).contains("unknown option"));
    }

    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_list_deployments_axhub(
        &temp,
        r#"{"items":[{"id":"dep-paydrop","app_id":"paydrop","status":"running","created_at":"2026-05-11T00:00:00Z"}]}"#,
        0,
    );
    let string_app = Command::new(bin())
        .args(["list-deployments", "--app", "paydrop"])
        .env("AXHUB_BIN", axhub.to_str().unwrap())
        .output()
        .unwrap();
    assert_eq!(string_app.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&string_app.stdout).contains("dep-paydrop"));

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
    assert!(session_stdout.contains("/axhub:setup"));
    assert!(session_stdout.contains("말씀해주세요"));
    assert!(session_stdout.contains("감사 로그"));
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
  echo "axhub 0.15.3 (commit fake, built fake, fake)"
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

#[cfg(unix)]
#[test]
fn cli_routing_stats_disabled_and_empty_outputs_are_stable() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    let disabled_plain = run_stdin(
        &["routing-stats"],
        "",
        &[
            ("XDG_STATE_HOME", state_s.as_str()),
            ("AXHUB_NO_AUDIT", "1"),
        ],
    );
    assert_eq!(disabled_plain.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&disabled_plain.stdout).contains("audit log 가 비활성이에요"));

    let disabled_json = run_stdin(
        &["routing-stats", "--json"],
        "",
        &[
            ("XDG_STATE_HOME", state_s.as_str()),
            ("AXHUB_NO_AUDIT", "1"),
        ],
    );
    assert_eq!(disabled_json.status.code(), Some(0));
    let parsed = stdout_json(&disabled_json);
    assert_eq!(parsed["audit_disabled"], true);

    let empty_plain = run_stdin(
        &["routing-stats", "--since", "1h"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(empty_plain.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_plain.stdout).contains("아직 audit 데이터가 없어요"));

    let empty_json = run_stdin(
        &["routing-stats", "--since", "1m", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(empty_json.status.code(), Some(0));
    let parsed = stdout_json(&empty_json);
    assert_eq!(parsed["total_prompts"], 0);
}

#[cfg(unix)]
#[test]
fn cli_routing_stats_help_and_invalid_args_are_stable() {
    let help = run_stdin(&["routing-stats", "--help"], "", &[]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("routing-stats"));

    for (args, expected) in [
        (&["routing-stats", "--unknown"][..], "알 수 없는 flag"),
        (
            &["routing-stats", "--top", "NaN"][..],
            "--top 은 숫자여야 해요",
        ),
        (
            &["routing-stats", "--since", "3w"][..],
            "duration 단위는 d/h/m 또는 'all' 만",
        ),
    ] {
        let output = run_stdin(args, "", &[]);
        assert_eq!(output.status.code(), Some(64), "args={args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "stderr={} expected={expected}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// Phase 7: SessionStart magical-moment + welcome marker.
//
// XDG_STATE_HOME=tempdir 으로 audit/welcome marker 격리. session-start 는 stdin
// 안 읽으니 run_stdin 의 stdin = "" 사용. systemMessage JSON parse 후 한국어
// 톤 / magical moment marker / marker file persistence 검증.

#[cfg(unix)]
fn session_start_systemmessage(state_s: &str) -> String {
    let output = run_stdin(
        &["session-start"],
        "",
        &[
            ("XDG_STATE_HOME", state_s),
            ("AXHUB_BIN", "/definitely/missing/axhub"),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // session-start 가 systemMessage JSON 1 line + meta_envelope JSON 1 line 출력. 첫 번째 만 추출.
    let first = stdout
        .lines()
        .find(|l| l.contains("systemMessage"))
        .expect("systemMessage line");
    let parsed: serde_json::Value = serde_json::from_str(first).expect("valid JSON");
    parsed["systemMessage"]
        .as_str()
        .expect("systemMessage is string")
        .to_owned()
}

#[cfg(unix)]
#[test]
fn cli_session_start_writes_session_bundle_cache() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();
    let config_s = temp.path().join("config").display().to_string();
    let cache_s = temp.path().join("cache").display().to_string();

    let output = run_stdin(
        &["session-start"],
        "",
        &[
            ("XDG_STATE_HOME", &state_s),
            ("XDG_CONFIG_HOME", &config_s),
            ("XDG_CACHE_HOME", &cache_s),
            ("AXHUB_BIN", "/definitely/missing/axhub"),
            ("AXHUB_APP_SLUG", "paydrop"),
            ("AXHUB_PROFILE", "prod"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle_path = temp.path().join("cache/axhub-plugin/session-bundle.json");
    let bundle = std::fs::read_to_string(&bundle_path)
        .unwrap_or_else(|err| panic!("missing session bundle at {bundle_path:?}: {err}"));
    let parsed: serde_json::Value = serde_json::from_str(&bundle).unwrap();
    assert_eq!(parsed["schema_version"], "session-bundle/v1");
    assert_eq!(parsed["auth_status"]["ok"], false);
    assert_eq!(parsed["auth_status"]["scopes"], serde_json::json!([]));
    assert_eq!(parsed["current_app"], "paydrop");
    assert_eq!(parsed["current_env"], "prod");
    assert_eq!(parsed["helper_version"], env!("CARGO_PKG_VERSION"));
}

#[cfg(unix)]
#[test]
fn cli_session_start_first_current_version_session() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    let msg = session_start_systemmessage(&state_s);
    let helper_version = env!("CARGO_PKG_VERSION");
    assert!(msg.contains(&format!("v{helper_version} 첫 세션")), "{msg}");
    assert!(msg.contains("감사 로그"), "{msg}");

    // Marker file 생성됐는지.
    let marker = state
        .join("axhub-plugin")
        .join(format!(".v{}-welcome-shown", env!("CARGO_PKG_VERSION")));
    assert!(marker.exists(), "marker missing at {marker:?}");
}

#[cfg(unix)]
#[test]
fn cli_session_start_subsequent_session() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let dir = state.join("axhub-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!(".v{}-welcome-shown", env!("CARGO_PKG_VERSION"))),
        "shown\n",
    )
    .unwrap();

    let msg = session_start_systemmessage(&state.display().to_string());
    assert!(
        !msg.contains(&format!("v{} 첫 세션", env!("CARGO_PKG_VERSION"))),
        "magical moment should not repeat: {msg}"
    );
}

#[cfg(unix)]
#[test]
fn cli_session_start_base_message_korean_tone() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");

    let msg = session_start_systemmessage(&state.display().to_string());
    assert!(msg.contains("axhub 준비됐어요"), "{msg}");
    assert!(msg.contains("/axhub:setup"), "{msg}");
    assert!(msg.contains("/axhub:help"), "{msg}");
    assert!(msg.contains("/axhub:doctor"), "{msg}");
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
        "context": {"source":"axhub.yaml"}
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

fn consent_mint_valid_binding(session_id: &str, tool_call_suffix: &str) -> String {
    serde_json::json!({
        "tool_call_id": format!("{session_id}:{tool_call_suffix}"),
        "action":"deploy_create",
        "app_id":"paydrop",
        "profile":"prod",
        "branch":"main",
        "commit_sha":"abc123",
        "context": {}
    })
    .to_string()
}

#[test]
fn cli_consent_mint_stdin_valid_json_behavior_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let runtime_dir = temp.path().join("runtime");
    let state = state_dir.display().to_string();
    let runtime = runtime_dir.display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "stdin-valid-session"),
    ];
    let session_token = runtime_dir.join("axhub/consent-stdin-valid-session.json");
    let binding = consent_mint_valid_binding("stdin-valid-session", "tc-valid");

    let minted = run_stdin(&["consent-mint"], &binding, &envs);
    assert_eq!(minted.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&minted.stdout);
    assert!(stdout.contains("token_id"), "{stdout}");
    assert!(stdout.contains("expires_at"), "{stdout}");
    assert!(session_token.exists());
}

#[test]
fn cli_consent_mint_stdin_empty_input_has_actionable_powershell_diagnostic() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let runtime_dir = temp.path().join("runtime");
    let state = state_dir.display().to_string();
    let runtime = runtime_dir.display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "stdin-empty-session"),
    ];

    for stdin in ["", "   \r\n\t", "\u{feff}\r\n\t "] {
        let rejected = run_stdin(&["consent-mint"], stdin, &envs);
        assert_ne!(rejected.status.code(), Some(0));
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(stderr.contains("empty stdin"), "{stderr}");
        assert!(stderr.contains("no JSON binding"), "{stderr}");
        assert!(stderr.contains("ConvertTo-Json -Compress"), "{stderr}");
        assert!(stderr.contains("$env:CLAUDE_PLUGIN_ROOT"), "{stderr}");
        assert!(
            stderr.contains(r#"& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" consent-mint"#),
            "{stderr}"
        );
        assert!(stderr.contains("Temp-file fallback"), "{stderr}");
        assert!(stderr.contains("Get-Content -Raw"), "{stderr}");
    }

    assert_no_consent_side_effects(&state_dir, &runtime_dir);
}

#[test]
fn cli_consent_mint_stdin_invalid_json_has_actionable_diagnostic_without_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let runtime_dir = temp.path().join("runtime");
    let state = state_dir.display().to_string();
    let runtime = runtime_dir.display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "stdin-invalid-session"),
    ];

    let rejected = run_stdin(&["consent-mint"], "{not-json", &envs);
    assert_ne!(rejected.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert!(stderr.contains("invalid JSON"), "{stderr}");
    assert!(stderr.contains("expects one JSON object"), "{stderr}");
    assert_no_consent_side_effects(&state_dir, &runtime_dir);
}

#[test]
fn cli_consent_mint_stdin_accepts_bom_and_whitespace_wrapped_json() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let runtime_dir = temp.path().join("runtime");
    let state = state_dir.display().to_string();
    let runtime = runtime_dir.display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("CLAUDE_SESSION_ID", "stdin-bom-session"),
    ];
    let session_token = runtime_dir.join("axhub/consent-stdin-bom-session.json");
    let binding = consent_mint_valid_binding("stdin-bom-session", "tc-bom");
    let wrapped_binding = format!("\u{feff}\r\n\t {binding}\n\n");

    let minted = run_stdin(&["consent-mint"], &wrapped_binding, &envs);
    assert_eq!(minted.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&minted.stdout);
    assert!(stdout.contains("token_id"), "{stdout}");
    assert!(session_token.exists());
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

    let apps_git_without_token = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"cli-e2e-session","tool_call_id":"tc-apps-git-deny","tool_name":"Bash","tool_input":{"command":"axhub apps git connect --app paydrop --repo jocoding/paydrop --branch main --execute --json"}}"#,
        &envs,
    );
    assert_eq!(apps_git_without_token.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&apps_git_without_token.stdout)
        .contains("permissionDecision\":\"deny"));

    let pending_github_binding = serde_json::json!({
        "tool_call_id":"pending",
        "action":"github_connect",
        "app_id":"paydrop",
        "profile":"",
        "branch":"main",
        "commit_sha":"",
        "context": {"repo":"jocoding/paydrop", "branch":"main"}
    })
    .to_string();
    let pending_github_minted =
        run_stdin(&["consent-mint"], &pending_github_binding, &pending_envs);
    assert_eq!(pending_github_minted.status.code(), Some(0));
    let pending_github_allowed = run_stdin(
        &["preauth-check"],
        r#"{"session_id":"actual-claude-session","tool_call_id":"toolu_apps_git","tool_name":"Bash","tool_input":{"command":"axhub apps git connect --app paydrop --repo jocoding/paydrop --branch main --execute --json"}}"#,
        &pending_envs,
    );
    assert_eq!(pending_github_allowed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&pending_github_allowed.stdout)
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
            "axhub.yaml",
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

// Phase 9 sub-task 9.2 — preflight hook examples 주입 (env-gated AXHUB_INJECT_EXAMPLES).

#[cfg(unix)]
#[test]
fn cli_prompt_route_examples_injected_when_env_set() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let input =
        serde_json::json!({"hook_event_name":"UserPromptSubmit","prompt":"배포해줘"}).to_string();

    let with_env = run_stdin(
        &["prompt-route"],
        &input,
        &[
            ("AXHUB_BIN", axhub.to_str().unwrap()),
            ("AXHUB_INJECT_EXAMPLES", "1"),
            ("AXHUB_NO_AUDIT", "1"),
        ],
    );
    assert_eq!(with_env.status.code(), Some(0));
    let stdout_with = String::from_utf8_lossy(&with_env.stdout);
    assert!(
        stdout_with.contains("AXHUB_INJECT_EXAMPLES enabled"),
        "{stdout_with}"
    );

    let without_env = run_stdin(
        &["prompt-route"],
        &input,
        &[
            ("AXHUB_BIN", axhub.to_str().unwrap()),
            ("AXHUB_NO_AUDIT", "1"),
        ],
    );
    assert_eq!(without_env.status.code(), Some(0));
    let stdout_without = String::from_utf8_lossy(&without_env.stdout);
    assert!(
        !stdout_without.contains("AXHUB_INJECT_EXAMPLES"),
        "default 시 examples marker 없어야 해요. {stdout_without}",
    );
}

// Phase 10 — clarify audit feedback + routing-stats --confused + routing-dashboard.

#[cfg(unix)]
#[test]
fn cli_routing_stats_skill_invoke() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    let output = run_stdin(
        &["routing-stats", "--since", "7d", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["total_prompts"], 0);
}

#[cfg(unix)]
#[test]
fn cli_audit_clarify_appends_record_then_confused_filter_returns_it() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    let clarify = run_stdin(
        &[
            "audit-clarify",
            "--hash",
            "sha256:test123",
            "--chosen",
            "deploy",
        ],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(clarify.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&clarify.stdout);
    assert!(stdout.contains("audit-clarify 기록"), "{stdout}");

    let stats = run_stdin(
        &["routing-stats", "--confused", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));
    let stats_stdout = String::from_utf8_lossy(&stats.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stats_stdout.trim()).expect("valid JSON");
    assert!(
        parsed["total_prompts"].as_u64().unwrap() >= 1,
        "{stats_stdout}"
    );
    let confused = parsed["confused_prompts"]
        .as_array()
        .expect("confused_prompts array");
    assert!(
        confused
            .iter()
            .any(|row| row["hash"] == "sha256:test123" && row["chosen_skill"] == "deploy"),
        "{stats_stdout}"
    );
}

#[cfg(unix)]
#[test]
fn cli_audit_clarify_prompt_hashes_locally_for_portable_skill_snippet() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    let clarify = run_stdin(
        &[
            "audit-clarify",
            "--prompt",
            "배포 로그 애매해",
            "--chosen",
            "logs",
        ],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(clarify.status.code(), Some(0));

    let stats = run_stdin(
        &["routing-stats", "--confused", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));
    let stats_stdout = String::from_utf8_lossy(&stats.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stats_stdout.trim()).expect("valid JSON");
    let confused = parsed["confused_prompts"]
        .as_array()
        .expect("confused prompts");
    assert!(
        confused.iter().any(|record| {
            record["chosen_skill"].as_str() == Some("logs")
                && record["hash"]
                    .as_str()
                    .is_some_and(|hash| hash.starts_with("sha256:"))
        }),
        "{stats_stdout}"
    );
}

#[cfg(unix)]
#[test]
fn cli_routing_dashboard_html_renders() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    let _ = run_stdin(
        &["audit-clarify", "--hash", "sha256:dash", "--chosen", "logs"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );

    let dash = run_stdin(
        &["routing-dashboard", "--html"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(dash.status.code(), Some(0));
    let html = String::from_utf8_lossy(&dash.stdout);
    assert!(html.contains("<!DOCTYPE html>"), "{html}");
    assert!(html.contains("axhub routing dashboard"), "{html}");
    assert!(html.contains("<table>"), "{html}");
    assert!(
        html.contains("total prompts</div><div class=\"stat-value\">0"),
        "clarify feedback sentinel should not inflate total prompt count: {html}"
    );
    assert!(
        html.contains("auth failed</div><div class=\"stat-value\">0"),
        "clarify feedback sentinel should not inflate auth failures: {html}"
    );
    assert!(
        html.contains("clarify invoked</div><div class=\"stat-value\">1"),
        "clarify feedback count should still be visible: {html}"
    );
    assert!(html.contains("Failing prompt hashes"), "{html}");
    assert!(html.contains("sha256:dash"), "{html}");
    assert!(html.contains("logs"), "chosen_skill row 보여야 함: {html}");
}

// Phase 25 PR 25.4 — trace CLI coverage. These cases keep the rust coverage
// gate honest for the new `trace --json` command and its Korean human output.

#[test]
fn cli_trace_requires_deploy_id() {
    let out = run(&["trace", "--json"]);
    assert_eq!(out.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--deploy-id"));
}

#[cfg(unix)]
fn fake_axhub_logs(temp: &tempfile::TempDir) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-logs");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
	if [ "$1 $2 $3" = "--json deploy logs" ]; then
	  echo '{"type":"log","message":"INFO build started"}'
	  echo '{"type":"log","message":"ERROR build command failed with exit code 1"}'
	  echo '{"type":"log","message":"WARN network timeout while fetching dependency"}'
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
fn fake_axhub_app_logs(
    temp: &tempfile::TempDir,
    name: &str,
    messages: &[&str],
) -> std::path::PathBuf {
    // R3γ: 현행 `axhub --json deploy logs` 를 흉내내는 fake — message 들을 NDJSON
    // (`{"type":"log","message":"..."}`) 한 줄씩 emit. messages 가 비면 아무것도
    // 출력 안 해서 runtime_log_unavailable 경로를 테스트해요. (message 에 따옴표 금지)
    let axhub = temp.path().join(name);
    let mut body = String::from("#!/bin/sh\nif [ \"$1 $2 $3\" = \"--json deploy logs\" ]; then\n");
    for m in messages {
        body.push_str(&format!(
            "  echo '{{\"type\":\"log\",\"message\":\"{m}\"}}'\n"
        ));
    }
    body.push_str("  exit 0\nfi\nexit 1\n");
    std::fs::write(&axhub, body).unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

#[cfg(unix)]
fn fake_slow_axhub_logs(temp: &tempfile::TempDir) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-logs-slow");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
	if [ "$1 $2 $3" = "--json deploy logs" ]; then
	  sleep 6
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
fn write_trace_deploy_events(state: &Path, deploy_id: &str) {
    write_trace_deploy_events_with_reason(state, deploy_id, "build command failed");
}

#[cfg(unix)]
fn write_trace_deploy_events_with_reason(state: &Path, deploy_id: &str, reason: &str) {
    let dir = state.join("axhub-plugin").join("deploy-events");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{deploy_id}.jsonl"));
    let body = format!(
        "{{\"schema_version\":\"deploy-event/v1\",\"deploy_id\":\"{deploy_id}\",\"ts\":\"2026-05-11T00:00:00.000Z\",\"phase\":\"preflight\",\"duration_ms\":10}}\n{{\"schema_version\":\"deploy-event/v1\",\"deploy_id\":\"{deploy_id}\",\"ts\":\"2026-05-11T00:00:01.000Z\",\"phase\":\"failed\",\"duration_ms\":20,\"reason\":\"{reason}\"}}\n"
    );
    std::fs::write(path, body).unwrap();
}

#[cfg(unix)]
fn fake_axhub_raw_logs(temp: &tempfile::TempDir, name: &str, lines: &[&str]) -> std::path::PathBuf {
    // NDJSON 이 아닌 raw 라인을 emit 하는 fake (파싱 실패 경로 테스트용, CR #7/#8).
    let axhub = temp.path().join(name);
    let mut body = String::from("#!/bin/sh\nif [ \"$1 $2 $3\" = \"--json deploy logs\" ]; then\n");
    for l in lines {
        body.push_str(&format!("  echo '{l}'\n"));
    }
    body.push_str("  exit 0\nfi\nexit 1\n");
    std::fs::write(&axhub, body).unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

#[cfg(unix)]
#[test]
fn cli_trace_json_reads_events_and_build_log_patterns() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-json";
    write_trace_deploy_events(&state, deploy_id);
    let axhub = fake_axhub_logs(&temp);
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            deploy_id,
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = stdout_json(&out);
    assert_eq!(json["deploy_id"], deploy_id);
    assert_eq!(json["last_phase"], "failed");
    assert_eq!(json["failure_reason"], "build command failed");
    assert_eq!(json["phase_durations"].as_array().unwrap().len(), 2);
    assert!(json["build_log_errors"].as_array().unwrap().len() >= 2);
    assert!(json["matched_patterns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "build_command_failed"));
    assert!(json["matched_patterns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "network_timeout"));
}

#[cfg(unix)]
#[test]
fn cli_trace_json_failure_reason_beats_benign_runtime() {
    // CR #1: 빌드 단계 실패의 authoritative 원인(event_log failure_reason)은 항상
    // 매칭되고, benign 런타임 로그 라인은 build-log needle 을 오발화하지 않아요.
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-reason";
    write_trace_deploy_events_with_reason(&state, deploy_id, "env: STRIPE_KEY not found");
    // 런타임 로그엔 ERROR/WARN 태그 없는 benign 라인만 (docker pull → 오발화 유혹).
    let axhub = fake_axhub_app_logs(
        &temp,
        "axhub-benign",
        &[
            "INFO docker pull completed successfully",
            "INFO listening on port 3000",
        ],
    );
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            deploy_id,
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = stdout_json(&out);
    let patterns = json["matched_patterns"].as_array().unwrap();
    // authoritative reason → env_not_found 매칭.
    assert!(
        patterns.iter().any(|v| v == "env_not_found"),
        "expected env_not_found (from failure_reason) in {patterns:?}"
    );
    // benign "docker pull completed" 가 docker_image_pull_failed 오발화 금지.
    assert!(
        !patterns.iter().any(|v| v == "docker_image_pull_failed"),
        "benign runtime line must NOT false-positive docker_image_pull_failed: {patterns:?}"
    );
}

#[cfg(unix)]
#[test]
fn cli_trace_json_matches_severity_tagged_runtime_error() {
    // CR #1: severity-tagged(ERROR/WARN) 런타임 라인은 매칭되고, benign INFO 라인
    // (`env: production`)은 env_not_found 오발화 안 해요.
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-runtime-err";
    write_trace_deploy_events_with_reason(&state, deploy_id, "container crashed");
    let axhub = fake_axhub_app_logs(
        &temp,
        "axhub-runtime",
        &["INFO env: production", "ERROR cannot find module 'vite'"],
    );
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            deploy_id,
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );
    assert_eq!(out.status.code(), Some(0));
    let json = stdout_json(&out);
    let patterns = json["matched_patterns"].as_array().unwrap();
    // tagged ERROR line → module_not_found.
    assert!(
        patterns.iter().any(|v| v == "module_not_found"),
        "expected module_not_found (tagged runtime line) in {patterns:?}"
    );
    // benign "INFO env: production" 가 env_not_found 오발화 금지.
    assert!(
        !patterns.iter().any(|v| v == "env_not_found"),
        "benign INFO env: line must NOT false-positive env_not_found: {patterns:?}"
    );
}

#[cfg(unix)]
#[test]
fn cli_trace_json_warns_on_unparseable_runtime_log() {
    // CR #7/#8: 런타임 로그가 NDJSON 이 아니면 runtime_log_unparseable warning 단일.
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-badjson";
    write_trace_deploy_events(&state, deploy_id);
    let axhub = fake_axhub_raw_logs(&temp, "axhub-raw", &["not json at all", "still not json"]);
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            deploy_id,
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );
    assert_eq!(out.status.code(), Some(0));
    let json = stdout_json(&out);
    assert!(
        json["warnings"].as_array().unwrap().iter().any(|v| v
            .as_str()
            .is_some_and(|s| s.starts_with("runtime_log_unparseable"))),
        "expected runtime_log_unparseable in {:?}",
        json["warnings"]
    );
}

#[cfg(unix)]
#[test]
fn cli_trace_json_warns_when_runtime_log_empty() {
    // R3γ T005: 런타임 로그가 비면 (빌드 단계 실패) runtime_log_unavailable warning +
    // event_log failure_reason 로 fallback 매칭.
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-empty";
    write_trace_deploy_events(&state, deploy_id);
    let axhub = fake_axhub_app_logs(&temp, "axhub-empty", &[]);
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            deploy_id,
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );
    assert_eq!(out.status.code(), Some(0));
    let json = stdout_json(&out);
    assert!(
        json["warnings"].as_array().unwrap().iter().any(|v| v
            .as_str()
            .is_some_and(|s| s.starts_with("runtime_log_unavailable"))),
        "expected runtime_log_unavailable in {:?}",
        json["warnings"]
    );
    // 빌드 단계 fallback: event_log reason("build command failed") 으로 매칭.
    assert!(
        json["matched_patterns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "build_command_failed"),
        "expected build_command_failed (event_log reason fallback) in {:?}",
        json["matched_patterns"]
    );
}

#[cfg(unix)]
#[test]
fn cli_trace_rejects_path_traversal_deploy_id() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let outside = state.join("axhub-plugin").join("probe.jsonl");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(
        &outside,
        r#"{"schema_version":"deploy-event/v1","deploy_id":"../probe","ts":"2026-05-11T00:00:00.000Z","phase":"failed","reason":"outside"}"#,
    )
    .unwrap();
    let axhub = fake_axhub_logs(&temp);
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            "../probe",
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );

    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid deploy_id"), "stderr={stderr}");
}

#[cfg(unix)]
#[test]
fn cli_trace_times_out_slow_build_log_probe() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-timeout";
    write_trace_deploy_events(&state, deploy_id);
    let axhub = fake_slow_axhub_logs(&temp);
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let started = std::time::Instant::now();
    let out = run_env(
        &[
            "trace",
            "--deploy-id",
            deploy_id,
            "--app",
            "paydrop",
            "--json",
        ],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );

    assert!(
        started.elapsed() < std::time::Duration::from_secs(7),
        "trace should enforce the 5s build-log timeout"
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = stdout_json(&out);
    // PR #149 / US-017: probe timeout WARN now surfaces on TraceReport.warnings
    // (separate channel) instead of polluting build_log_errors. SKILL parsers
    // branch on warnings; build_log_errors stays evidence-only.
    assert!(
        json["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str().unwrap().contains("runtime_log_probe_timeout")),
        "{json}",
    );
    assert!(
        json["build_log_errors"].as_array().unwrap().is_empty(),
        "build_log_errors must not carry probe-side timeout WARN: {json}"
    );
}

#[cfg(unix)]
#[test]
fn cli_trace_human_output_includes_phase_errors_and_patterns() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let deploy_id = "dep-cli-trace-human";
    write_trace_deploy_events(&state, deploy_id);
    let axhub = fake_axhub_logs(&temp);
    let state_s = state.display().to_string();
    let axhub_s = axhub.display().to_string();

    let out = run_env(
        &["trace", "--deploy-id", deploy_id, "--app", "paydrop"],
        &[("XDG_STATE_HOME", &state_s), ("AXHUB_BIN", &axhub_s)],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("마지막 phase: failed"), "stdout={stdout}");
    assert!(
        stdout.contains("실패 사유: build command failed"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("build_log 마지막"), "stdout={stdout}");
    assert!(stdout.contains("매칭 패턴"), "stdout={stdout}");
}

#[test]
fn cli_migrate_plan_detects_node_env_and_manifest_snippet() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"scripts":{"start":"vite --host 0.0.0.0"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("src/main.ts"),
        "console.log(process.env.DATABASE_URL, process.env['VITE_PUBLIC_URL']);",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/settings.py"),
        "import os\nDATABASE_URL = os.environ['PY_DATABASE_URL']\n",
    )
    .unwrap();

    let output = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], "migrate-plan/v1");
    assert_eq!(json["monorepo"], false);
    assert_eq!(json["container_contracts"]["dockerfile"], false);
    assert_eq!(json["container_contracts"]["compose"], false);
    assert_eq!(json["candidates"][0]["stack_hint"], "node");
    assert!(json["candidates"][0]["env_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "DATABASE_URL"));
    assert!(json["suggested_manifest"]
        .as_str()
        .unwrap()
        .contains("axhub/v1"));
    let env_refs = json["env_refs"].as_array().unwrap();
    assert!(env_refs
        .iter()
        .any(|e| e["name"] == "DATABASE_URL" && e["scope"] == "runtime"));
    assert!(env_refs
        .iter()
        .any(|e| e["name"] == "VITE_PUBLIC_URL" && e["scope"] == "build"));
    assert!(env_refs
        .iter()
        .any(|e| e["name"] == "PY_DATABASE_URL" && e["scope"] == "runtime"));
}

#[test]
fn cli_migrate_plan_detects_monorepo_candidates_and_compose() {
    let temp = tempfile::tempdir().unwrap();
    let web = temp.path().join("apps/web");
    std::fs::create_dir_all(&web).unwrap();
    std::fs::write(web.join("package.json"), "{}").unwrap();
    std::fs::write(
        web.join("compose.yaml"),
        "services:\n  web:\n    build: .\n",
    )
    .unwrap();
    let api = temp.path().join("services/api");
    std::fs::create_dir_all(&api).unwrap();
    std::fs::write(api.join("go.mod"), "module example.com/api\n").unwrap();
    std::fs::write(api.join("Dockerfile"), "FROM scratch\n").unwrap();
    std::fs::write(api.join("main.go"), "package main\n// API_DSN\n").unwrap();

    let output = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["monorepo"], true);
    assert_eq!(json["container_contracts"]["compose"], true);
    let candidates = json["candidates"].as_array().unwrap();
    assert!(candidates.iter().any(|c| c["path"] == "apps/web"
        && c["has_compose"] == true
        && c["compose_file"] == "compose.yaml"));
    assert!(candidates
        .iter()
        .any(|c| c["path"] == "services/api" && c["stack_hint"] == "go"));

    let selected_output = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--app-path",
        "services/api",
        "--json",
    ]);
    assert_eq!(selected_output.status.code(), Some(0));
    let selected_json: serde_json::Value = serde_json::from_slice(&selected_output.stdout).unwrap();
    assert!(selected_json["suggested_manifest"]
        .as_str()
        .unwrap()
        .contains("dockerfile: \"services/api/Dockerfile\""));
    assert!(!selected_json["suggested_manifest"]
        .as_str()
        .unwrap()
        .contains("apps/web/compose.yaml"));
}

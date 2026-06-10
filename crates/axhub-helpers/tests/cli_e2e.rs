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
    // session-start's drift-cache warm only fires under CLAUDE_PLUGIN_ROOT (real
    // plugin session). Strip it so the suite never spawns a network fetch even if
    // a dev runs `cargo test` with it exported.
    command.env_remove("CLAUDE_PLUGIN_ROOT");
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
  echo "axhub 0.17.3"
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
  echo "axhub 0.17.3"
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
  echo "axhub 0.17.3"
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
  echo axhub 0.17.3
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
    let axhub = fake_deploy_prep_axhub(&temp, "axhub 0.17.3");

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

    let classified = run(&["classify-exit", "--exit-code", "4", "--stdout", "{}"]);
    assert!(classified.status.success());
    assert!(String::from_utf8_lossy(&classified.stdout).contains("로그인이 만료"));

    // spec 004: a helper-output exit (65 = EXIT_LIST_AUTH / preflight::EXIT_AUTH)
    // must normalize to the same auth template at the binary level, not only in
    // the catalog.rs unit test.
    let helper_auth = run(&["classify-exit", "--exit-code", "65", "--stdout", "{}"]);
    assert!(helper_auth.status.success());
    assert!(String::from_utf8_lossy(&helper_auth.stdout).contains("로그인이 만료"));
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
        "tool_response": {"exit_code": 4, "stdout": "{}"}
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
        "tool_response": {"exit_code": 4, "stdout": "{}"}
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
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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

    // Approach E (Phase 2): Rust router does NOT classify generic intent.
    // cmd_prompt_route emits preflight context regardless of utterance. Skill
    // matching happens via Claude Code's native description matching, except for
    // the narrow dynamic-table anti-detour hint covered below.
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
        "testnextjs 앱 잠깐 멈춰줘",
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

#[cfg(unix)]
#[test]
fn cli_prompt_route_onboarding_prompt_forces_onboarding_skill_contract() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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

    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "처음인데 뭐부터 하면 돼?",
    })
    .to_string();
    let output = run_stdin(
        &["prompt-route"],
        &input,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));

    let stdout_json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("hook JSON");
    let ctx = stdout_json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("additionalContext");
    let system = stdout_json["systemMessage"].as_str().unwrap_or("");
    let combined = format!("{ctx}\n{system}");
    assert!(ctx.contains("Skill(\"axhub:onboarding\")"), "{ctx}");
    // routing control rides additionalContext (agent-facing); it must NOT leak
    // into the user-visible systemMessage.
    assert!(!system.contains("Skill(\"axhub:onboarding\")"), "{system}");
    assert!(
        combined.contains("Skill(\"axhub:onboarding\")"),
        "{combined}"
    );
    assert!(
        combined.contains("never announce internal routing"),
        "{combined}"
    );
    assert!(ctx.contains("내부 제어"), "{ctx}");
    assert!(ctx.contains("visible chat 에 절대 쓰지 않아요"), "{ctx}");
    assert!(combined.contains("first-run onboarding"), "{combined}");
    assert!(combined.contains("첫 앱 만들래요?"), "{combined}");
    assert!(
        combined.contains("do not ask template choices first"),
        "{combined}"
    );
    assert!(!combined.contains("새 앱 만들어줘\" 하면 됨"), "{combined}");
    assert!(
        !combined.contains("I'll invoke the onboarding skill"),
        "{combined}"
    );
}

#[cfg(unix)]
#[test]
fn cli_prompt_route_dynamic_table_hint_is_surgical() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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

    let table_input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "ultraqa-app 앱에 orders 동적 테이블 만들고 title:text 컬럼 추가해",
    })
    .to_string();
    let output = run_stdin(
        &["prompt-route"],
        &table_input,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout_json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("hook JSON");
    let ctx = stdout_json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("additionalContext");
    assert!(ctx.contains("<axhub-routing-hint>"), "{ctx}");
    assert!(ctx.contains("테이블 변경 내용을 확인할게요"), "{ctx}");
    assert!(ctx.contains("Korean preview of the target app"), "{ctx}");
    assert!(!ctx.contains("Skill(axhub:"), "{ctx}");
    assert!(
        ctx.contains("must not include raw CLI command lines"),
        "{ctx}"
    );
    assert!(ctx.contains("do not call AskUserQuestion"), "{ctx}");
    assert!(ctx.contains("raw question JSON"), "{ctx}");
    assert!(!ctx.contains("AXHub tables workflow"), "{ctx}");
    assert!(!ctx.contains("axhub tables create \"$TABLE\""), "{ctx}");
    assert!(
        !ctx.contains("axhub tables columns add \"$TABLE\""),
        "{ctx}"
    );
    assert!(!ctx.contains(&["consent", "-mint"].concat()), "{ctx}");

    let data_input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "describe snowflake analytics orders table",
    })
    .to_string();
    let data_output = run_stdin(
        &["prompt-route"],
        &data_input,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(data_output.status.code(), Some(0));
    let data_stdout = String::from_utf8_lossy(&data_output.stdout);
    assert!(
        data_stdout.contains("데이터 리소스를 확인할게요"),
        "catalog/data prompt should get the data hint: {data_stdout}"
    );
    assert!(
        !data_stdout.contains("테이블 변경 내용을 확인할게요"),
        "catalog/data prompt must not get dynamic-table hint: {data_stdout}"
    );
}

#[cfg(unix)]
#[test]
fn cli_prompt_route_desktop_app_template_hints_are_surgical() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let cases = [
        ("새 앱 만들어줘", "AXHub app creation request"),
        ("내 앱 목록 보여줘", "현재 팀 scope"),
        ("템플릿 뭐 있어?", "read-only"),
    ];

    for (prompt, expected) in cases {
        let input = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": prompt,
        })
        .to_string();
        let output = run_stdin(
            &["prompt-route"],
            &input,
            &[
                ("AXHUB_BIN", axhub.to_str().unwrap()),
                ("AXHUB_NO_AUDIT", "1"),
            ],
        );
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected),
            "{prompt:?} should inject {expected}; stdout={stdout}"
        );
        assert!(
            !stdout.contains("skills/") && !stdout.contains("SKILL.md"),
            "hints must not regress to path enforcement: {stdout}"
        );
        assert!(
            !stdout.contains("Skill(axhub:") && !stdout.contains("skill trigger"),
            "hints must not leak internal skill labels into model-visible prose: {stdout}"
        );
        if prompt == "새 앱 만들어줘" {
            let stdout_json: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("hook JSON");
            let additional_context = stdout_json["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .expect("additionalContext");
            let system_message = stdout_json["systemMessage"].as_str().unwrap_or("");
            assert!(
                additional_context.contains("Skill(\"axhub:init\")"),
                "new-app Desktop context must route through the native init skill surface: {additional_context}"
            );
            // routing control rides additionalContext (agent-facing); it must NOT
            // leak into the user-visible systemMessage.
            assert!(
                !system_message.contains("Skill(\"axhub:init\")"),
                "new-app Desktop hint must keep internal routing out of systemMessage: {stdout}"
            );
            assert!(
                stdout.contains("브레인스토밍, 일반 프로젝트 탐색, 또는 앱 아이디어 분류가 아니라 AXHub 앱 생성 절차"),
                "new-app Desktop hint must steer away from brainstorming/generic discovery: {stdout}"
            );
            assert!(
                stdout.contains("Do not add an explicit 기타 option"),
                "new-app Desktop hint must avoid duplicate Other options: {stdout}"
            );
            assert!(
                stdout.contains("첫 문장은 정확히"),
                "new-app Desktop hint must keep internal labels out of user-facing text: {stdout}"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn cli_prompt_route_status_hint_prevents_memory_answer() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ] && [ "$3" = "--json" ]; then
  echo '{"status":"error","error":{"code":"auth","subcode":"token_missing"}}'
  exit 65
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "어디까지 됐어",
    })
    .to_string();
    let output = run_stdin(
        &["prompt-route"],
        &input,
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout_json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("hook JSON");
    let ctx = stdout_json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("additionalContext");
    assert!(ctx.contains("<axhub-routing-hint>"), "{ctx}");
    assert!(ctx.contains("AXHub status workflow"), "{ctx}");
    assert!(!ctx.contains("Skill(axhub:"), "{ctx}");
    assert!(ctx.contains("do not answer from repo/git memory"), "{ctx}");
    assert!(ctx.contains("로그인/토큰 확인"), "{ctx}");
    let system_message = stdout_json["systemMessage"].as_str().unwrap_or("");
    // the status route contract now rides additionalContext (agent-facing), not
    // the user-visible systemMessage.
    assert!(ctx.contains("배포 상태 요청"), "{ctx}");
    assert!(
        !system_message.contains("배포 상태 요청"),
        "{system_message}"
    );
}

#[cfg(unix)]
#[test]
fn cli_prompt_route_deploy_and_doctor_hints_prevent_repo_answers() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
case "$1" in
  --version) printf '0.17.3\n' ;;
  auth) printf '{"authenticated":false}\n' ;;
  *) printf '{}\n' ;;
esac
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    for (prompt, workflow, phrase, user_phrase) in [
        (
            "배포해",
            "First visible sentence, exactly",
            "axhub-helpers deploy-preview-summary --user-utterance",
            "배포 준비 확인",
        ),
        (
            "환경 점검해",
            "First visible sentence, exactly",
            "axhub-helpers doctor-summary --user-utterance",
            "설치 상태 확인",
        ),
        (
            "axhub 앱이 어떤 API 쓸 수 있는지 보여줘",
            "AXHub API catalog workflow",
            "axhub catalog resources --json --limit 50",
            "API 카탈로그",
        ),
        (
            "매니페스트랑 설정 괜찮은지 봐줘",
            "First visible sentence, exactly",
            "axhub-helpers inspect-config-summary",
            "매니페스트와 설정 확인",
        ),
        (
            "로그 좀 보여줘",
            "First visible sentence, exactly",
            "axhub-helpers logs-summary --user-utterance",
            "로그 확인",
        ),
        (
            "라이브 페이지 열어봐",
            "First visible sentence, exactly",
            "axhub-helpers open-summary --user-utterance",
            "앱 페이지 확인",
        ),
        (
            "방금 배포 진짜 열리는지 확인해줘",
            "First visible sentence, exactly",
            "axhub-helpers verify-summary --user-utterance",
            "배포 검증",
        ),
        (
            "배포 실패 원인 알려줘",
            "First visible sentence, exactly",
            "axhub-helpers trace-summary --user-utterance",
            "배포 기록 확인",
        ),
        (
            "방금 배포 되돌려줘",
            "First visible sentence, exactly",
            "axhub-helpers rollback-summary --user-utterance",
            "배포 되돌리기 확인",
        ),
        (
            "이번 주 axhub 라우팅 어땠어?",
            "First visible sentence, exactly",
            "axhub-helpers routing-stats --since 7d",
            "라우팅 통계 확인",
        ),
        (
            "환경변수 뭐 있어?",
            "First visible sentence, exactly",
            "axhub-helpers env-summary --user-utterance",
            "환경변수 확인",
        ),
        (
            "Postgres 데이터베이스 연결하고 싶어",
            "AXHub external database connector",
            "Do not inspect or edit local app code",
            "데이터베이스 연결을 준비할게요",
        ),
        (
            "리소스 정리하고 싶어",
            "First visible sentence, exactly",
            "axhub-helpers resources-summary --user-utterance",
            "리소스 현황 확인",
        ),
        (
            "이 앱 깃허브랑 연결돼 있어?",
            "First visible sentence, exactly",
            "axhub-helpers github-summary --user-utterance",
            "GitHub 연결 상태 확인",
        ),
        (
            "이 프로젝트 axhub로 옮길 수 있어?",
            "First visible sentence, exactly",
            "axhub-helpers migrate-summary --user-utterance",
            "가져오기 상태 확인",
        ),
        (
            "이 앱 공개 심사 넣고 싶어",
            "First visible sentence, exactly",
            "axhub-helpers publish-summary --user-utterance",
            "공개 심사 준비 확인",
        ),
        (
            "팀원 초대해",
            "First visible sentence, exactly",
            "axhub-helpers team-summary --user-utterance",
            "팀 작업 확인",
        ),
        (
            "testnextjs 앱 잠깐 멈춰줘",
            "AXHub hosted app lifecycle request",
            "앱 변경을 실행할까요?",
            "앱 변경 실행",
        ),
        (
            "testnextjs 다시 켜줘",
            "AXHub hosted app lifecycle request",
            "앱 변경을 실행할까요?",
            "앱 변경 실행",
        ),
        (
            "axhub CLI 설치 상태 괜찮아?",
            "First visible sentence, exactly",
            "axhub-helpers doctor-summary --user-utterance",
            "설치 상태 확인",
        ),
        (
            "axhub CLI 설치해줘",
            "First visible sentence, exactly",
            "axhub-helpers install-summary --user-utterance",
            "설치 상태 확인",
        ),
        (
            "업데이트 필요한지 봐줘",
            "First visible sentence, exactly",
            "axhub-helpers update-summary --user-utterance",
            "업데이트 확인",
        ),
        (
            "나 로그인 돼 있어?",
            "로그인 상태를 확인할게요",
            "axhub-helpers auth-summary --user-utterance",
            "로그인 상태 확인",
        ),
        (
            "로그인 다시 해야 해?",
            "로그인 상태를 확인할게요",
            "axhub-helpers auth-summary --user-utterance",
            "로그인 상태 확인",
        ),
        (
            "상태바 켜줘",
            "First visible sentence, exactly",
            "axhub-helpers statusline-summary --user-utterance",
            "상태바 설정",
        ),
        (
            "axhub 좀 도와줘",
            "First visible sentence, exactly",
            "어떤 걸 도와드릴까요?",
            "작업 선택",
        ),
    ] {
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
        assert_eq!(output.status.code(), Some(0), "{prompt}");
        let stdout_json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("hook JSON");
        let ctx = stdout_json["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .expect("additionalContext");
        assert!(ctx.contains("<axhub-routing-hint>"), "{ctx}");
        assert!(ctx.contains(workflow), "{ctx}");
        assert!(!ctx.contains("Skill(axhub:"), "{ctx}");
        assert!(ctx.contains(phrase), "{ctx}");
        // The whole route contract — first-sentence cue, helper command, and
        // guardrail prose — now ships in additionalContext (agent-facing); the
        // user-facing systemMessage carries only the grace nudge (empty here:
        // unauthed). The per-prompt content assertions below check the contract
        // on additionalContext, the channel it now rides, so `system_message` is
        // aliased to `ctx`. `system_message_raw` holds the ACTUAL systemMessage
        // for the checks that assert the user-facing channel stays empty/clean.
        let system_message_raw = stdout_json["systemMessage"].as_str().unwrap_or("");
        let system_message = ctx;
        assert!(ctx.contains(user_phrase), "{ctx}");
        if prompt == "매니페스트랑 설정 괜찮은지 봐줘" {
            assert!(!ctx.contains("AXHub inspect summary helper"), "{ctx}");
            assert!(
                !system_message.contains("helper 로 처리"),
                "{system_message}"
            );
            assert!(
                !system_message.contains("점검 요청이에요"),
                "{system_message}"
            );
        }
        if prompt == "로그 좀 보여줘" {
            assert!(ctx.contains("Do not inspect local repo log files"), "{ctx}");
            assert!(
                system_message.contains("로컬 파일 로그"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "배포해" {
            assert!(
                !ctx.contains("Observed: axhub deploy/create prompt"),
                "{ctx}"
            );
            assert!(
                !ctx.contains("Suggested: use the AXHub deploy workflow"),
                "{ctx}"
            );
            assert!(
                system_message.contains("visible chat 으로 정확히 \"배포 준비를 확인할게요.\""),
                "{system_message}"
            );
            assert!(
                system_message.contains("deploy-preview-summary"),
                "{system_message}"
            );
            assert!(
                system_message.contains("deploy-approved-run"),
                "{system_message}"
            );
            assert!(
                system_message
                    .contains("preview 전에는 긴 deploy skill 본문을 읽거나 요약하지 않아요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("승인 후에는 skill 을 다시 호출하거나"),
                "{system_message}"
            );
        }
        if prompt == "라이브 페이지 열어봐" {
            assert!(ctx.contains("Do not inspect QA result files"), "{ctx}");
            assert!(ctx.contains("ToolSearch narration"), "{ctx}");
            assert!(system_message.contains("QA 결과 파일"), "{system_message}");
            assert!(
                system_message.contains("Chrome MCP 상태"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "방금 배포 진짜 열리는지 확인해줘" {
            assert!(ctx.contains("Do not narrate routing"), "{ctx}");
            assert!(ctx.contains("stale cache IDs"), "{ctx}");
            assert!(
                system_message.contains("배포가 실제로 열리는지 확인할게요"),
                "{system_message}"
            );
            assert!(system_message.contains("user email"), "{system_message}");
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "배포 실패 원인 알려줘" {
            assert!(ctx.contains("deployment failure-cause request"), "{ctx}");
            assert!(ctx.contains("Do not narrate routing"), "{ctx}");
            assert!(ctx.contains("failure_reason"), "{ctx}");
            assert!(ctx.contains("matched_patterns"), "{ctx}");
            assert!(
                system_message.contains("배포 기록을 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("trace-summary --user-utterance"),
                "{system_message}"
            );
            assert!(system_message.contains("deploy id"), "{system_message}");
            assert!(
                system_message.contains("failure_reason"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "방금 배포 되돌려줘" {
            assert!(
                ctx.contains("deployment restore/rollback/recover request"),
                "{ctx}"
            );
            assert!(ctx.contains("rollback-summary --user-utterance"), "{ctx}");
            assert!(
                system_message.contains("되돌릴 수 있는 배포를 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("배포 되돌리기 확인"),
                "{system_message}"
            );
            assert!(
                system_message.contains("rollback-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("명시적으로 승인하기 전에는 실행하지 않아요"),
                "{system_message}"
            );
            for leaked in ["스킬 호출", "/axhub:rollback", "/axhub:recover"] {
                assert!(
                    !system_message.contains(leaked),
                    "{leaked}: {system_message}"
                );
            }
        }
        if prompt == "이번 주 axhub 라우팅 어땠어?" {
            assert!(ctx.contains("Do not inspect QA result files"), "{ctx}");
            assert!(ctx.contains("desktop QA logs"), "{ctx}");
            assert!(system_message.contains("QA 결과 파일"), "{system_message}");
            assert!(
                system_message.contains("라우팅 통계를 확인할게요"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "환경변수 뭐 있어?" {
            assert!(
                ctx.contains("Do not inspect shell environment variables"),
                "{ctx}"
            );
            assert!(ctx.contains("raw values"), "{ctx}");
            assert!(system_message.contains("환경변수를 확인할게요"));
            assert!(system_message.contains(".env 파일"), "{system_message}");
            assert!(system_message.contains("raw value"), "{system_message}");
            assert!(system_message.contains("preflight"), "{system_message}");
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "Postgres 데이터베이스 연결하고 싶어" {
            assert!(ctx.contains("server.js"), "{ctx}");
            assert!(ctx.contains("package.json"), "{ctx}");
            assert!(ctx.contains("DATABASE_URL"), "{ctx}");
            assert!(
                ctx.contains("Do not ask for secret values in chat"),
                "{ctx}"
            );
            assert!(
                system_message.contains("로컬 앱 코드 수정"),
                "{system_message}"
            );
            assert!(
                system_message.contains("데이터베이스 연결을 준비할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("workflow/워크플로"),
                "{system_message}"
            );
            assert!(system_message.contains("계정 이메일"), "{system_message}");
            assert!(
                system_message.contains("A/B 구현 분기 라벨"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "이 프로젝트 axhub로 옮길 수 있어?" {
            assert!(
                ctx.contains("existing-app import/migration readiness"),
                "{ctx}"
            );
            assert!(
                ctx.contains("Do not answer from local server checks"),
                "{ctx}"
            );
            assert!(ctx.contains("previous deployment failure state"), "{ctx}");
            assert!(
                system_message.contains("가져오기 상태를 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("migrate-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("로컬 서버 점검"),
                "{system_message}"
            );
            assert!(
                system_message.contains("이전 배포 실패 상태"),
                "{system_message}"
            );
            assert!(
                system_message.contains("raw deploy status field"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "이 앱 공개 심사 넣고 싶어" {
            assert!(
                ctx.contains("marketplace/public review submission"),
                "{ctx}"
            );
            assert!(ctx.contains("Do not read quality files"), "{ctx}");
            assert!(
                system_message.contains("공개 심사 준비를 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("publish-summary --user-utterance"),
                "{system_message}"
            );
            assert!(system_message.contains("quality.json"), "{system_message}");
            assert!(
                system_message.contains("명시적 승인 전에는 실행하지 않아요"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "팀원 초대해" {
            assert!(ctx.contains("workspace team invitation"), "{ctx}");
            assert!(
                ctx.contains("Do not reinterpret it as Claude/OMC multi-agent team setup"),
                "{ctx}"
            );
            assert!(
                system_message.contains("팀 작업을 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("team-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("Claude/OMC 멀티에이전트 작업 팀"),
                "{system_message}"
            );
            assert!(
                system_message.contains("명시적 승인 전에는 실행하지 않아요"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "testnextjs 앱 잠깐 멈춰줘" || prompt == "testnextjs 다시 켜줘" {
            assert!(
                ctx.contains("Do not inspect local Next.js/dev-server processes"),
                "{ctx}"
            );
            assert!(ctx.contains("ps/lsof"), "{ctx}");
            assert!(ctx.contains("Continue in this same answer flow"), "{ctx}");
            assert!(ctx.contains("Human-visible flow"), "{ctx}");
            assert!(
                ctx.contains("Use exactly one `앱 변경 실행` Bash tool call"),
                "{ctx}"
            );
            assert!(ctx.contains("Never say `User chose`"), "{ctx}");
            assert!(ctx.contains("raw JSON stdout"), "{ctx}");
            assert!(ctx.contains("--execute --json >/dev/null"), "{ctx}");
            assert!(
                ctx.contains("Do not verify by re-running the mutation"),
                "{ctx}"
            );
            assert!(ctx.contains("비공개 (private)"), "{ctx}");
            assert!(ctx.contains("internal safety check blocks"), "{ctx}");
            assert!(ctx.contains("앱을 한 번 더 확인할게요"), "{ctx}");
            assert!(
                system_message.contains("로컬 Next.js/dev-server 프로세스"),
                "{system_message}"
            );
            assert!(
                system_message
                    .contains("이 대화 안에서 바로 진행하고 slash command 를 호출하지 않아요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("Bash tool call 하나로 matching top-level"),
                "{system_message}"
            );
            assert!(
                system_message.contains("User chose`, `execute suspend"),
                "{system_message}"
            );
            assert!(system_message.contains("계정 이메일"), "{system_message}");
            assert!(
                system_message.contains("JSON 을 직접 만들지 않고"),
                "{system_message}"
            );
            assert!(
                system_message.contains("내부 안전 점검이 막으면"),
                "{system_message}"
            );
            assert!(
                system_message.contains("raw JSON stdout"),
                "{system_message}"
            );
            assert!(
                system_message.contains("--execute --json >/dev/null"),
                "{system_message}"
            );
            assert!(
                system_message.contains("같은 top-level 명령을 한 번만 재시도해요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("비공개 (private)"),
                "{system_message}"
            );
            assert!(
                system_message.contains("식별자 조회를 설명하지 않아요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("진행`을 고르기 전에는 앱 상태를 바꾸지 않아요"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
            assert!(
                !system_message.contains("ID를 확인할게요"),
                "{system_message}"
            );
            assert!(!system_message.contains("승인 토큰"), "{system_message}");
            assert!(
                !system_message.contains("app-lifecycle skill"),
                "{system_message}"
            );
            assert!(!ctx.contains("app-lifecycle skill"), "{ctx}");
            assert!(
                !system_message.contains("/axhub:app-lifecycle"),
                "{system_message}"
            );
            assert!(!ctx.contains("/axhub:app-lifecycle"), "{ctx}");
        }
        if prompt == "testnextjs 다시 켜줘" {
            assert!(
                ctx.contains("resume intent: <app> 앱을 다시 켤 준비를 할게요."),
                "{ctx}"
            );
            assert!(
                system_message.contains("resume 은 `<앱 이름> 앱을 다시 켤 준비를 할게요.`"),
                "{system_message}"
            );
        }
        if prompt == "axhub CLI 설치 상태 괜찮아?" {
            assert!(ctx.contains("Skill(\"axhub:doctor\")"), "{ctx}");
            assert!(
                system_message.contains("Skill(\"axhub:doctor\")"),
                "{system_message}"
            );
            assert!(
                ctx.contains("Do not install, update, login, logout"),
                "{ctx}"
            );
            assert!(ctx.contains("raw user emails"), "{ctx}");
            assert!(
                system_message.contains("설치 상태를 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("doctor-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("raw user email"),
                "{system_message}"
            );
            assert!(system_message.contains("preflight"), "{system_message}");
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "axhub CLI 설치해줘" {
            assert!(ctx.contains("install-summary --user-utterance"), "{ctx}");
            assert!(
                system_message.contains("설치 상태를 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("install-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("installer 를 실행하지 않아요"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "업데이트 필요한지 봐줘" {
            assert!(ctx.contains("update-summary --user-utterance"), "{ctx}");
            assert!(ctx.contains("has_update"), "{ctx}");
            assert!(
                system_message.contains("업데이트를 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("update-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("명시적으로 적용을 승인하기 전에는"),
                "{system_message}"
            );
            for leaked in [
                "Check axhub update",
                "doctor-summary --user-utterance",
                "install-summary --user-utterance",
                "스킬 호출",
            ] {
                assert!(
                    !system_message.contains(leaked),
                    "{leaked}: {system_message}"
                );
            }
        }
        if prompt == "나 로그인 돼 있어?" || prompt == "로그인 다시 해야 해?" {
            assert!(ctx.contains("로그인 상태를 확인할게요"), "{ctx}");
            assert!(ctx.contains("auth-summary --user-utterance"), "{ctx}");
            assert!(ctx.contains("다시 로그인 필요 여부만 확인"), "{ctx}");
            assert!(
                ctx.contains("설치 상태 점검, 환경 진단, 업데이트 확인"),
                "{ctx}"
            );
            assert!(system_message_raw.is_empty(), "{system_message_raw}");
            for leaked in [
                "계정·만료·scope",
                "Check axhub auth status",
                "show account/expiry/scopes",
                "doctor-summary --user-utterance",
                "setup-summary",
                "logs-summary --user-utterance",
                "Do not route",
                "slash command",
                "skill name",
                "preflight",
                "axhub hook",
                "Control only",
                "스킬 호출",
            ] {
                assert!(!ctx.contains(leaked), "ctx leaked {leaked}: {ctx}");
                assert!(
                    !system_message_raw.contains(leaked),
                    "systemMessage leaked {leaked}: {system_message_raw}"
                );
            }
        }
        if prompt == "상태바 켜줘" {
            assert!(ctx.contains("Preserve an existing third-party"), "{ctx}");
            assert!(ctx.contains("existing command strings"), "{ctx}");
            assert!(
                system_message.contains("상태바 설정을 확인할게요"),
                "{system_message}"
            );
            assert!(
                system_message.contains("statusline-summary --user-utterance"),
                "{system_message}"
            );
            assert!(
                system_message.contains("기존 다른 상태바"),
                "{system_message}"
            );
            assert!(
                system_message.contains("기존 command 문자열"),
                "{system_message}"
            );
            assert!(!system_message.contains("스킬 호출"), "{system_message}");
        }
        if prompt == "axhub 좀 도와줘" {
            assert!(ctx.contains("Use exactly one question card"), "{ctx}");
            assert!(ctx.contains("환경 점검"), "{ctx}");
            assert!(ctx.contains("앱 배포"), "{ctx}");
            assert!(ctx.contains("앱과 리소스 조회"), "{ctx}");
            assert!(ctx.contains("문제 원인 보기"), "{ctx}");
            assert!(ctx.contains("처음부터 안내"), "{ctx}");
            assert!(ctx.contains("hidden option values"), "{ctx}");
            assert!(ctx.contains("do not call the Claude Skill tool"), "{ctx}");
            assert!(ctx.contains("doctor-summary --user-utterance"), "{ctx}");
            assert!(
                system_message.contains("어떤 걸 도와드릴까요?"),
                "{system_message}"
            );
            assert!(system_message.contains("작업 선택"), "{system_message}");
            assert!(
                system_message.contains("Claude Skill tool"),
                "{system_message}"
            );
            assert!(
                system_message.contains("doctor-summary --user-utterance"),
                "{system_message}"
            );
            for leaked in [
                "(doctor)",
                "(deploy)",
                "(status)",
                "(logs)",
                "(apps)",
                "/axhub:",
                "스킬 호출",
                "skill trigger",
                "\"value\": \"doctor\"",
                "\"value\": \"deploy\"",
                "너무 막연",
            ] {
                assert!(!ctx.contains(leaked), "ctx leaked {leaked}: {ctx}");
                assert!(
                    !system_message.contains(leaked),
                    "system leaked {leaked}: {system_message}"
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn deploy_preview_summary_outputs_human_card_without_raw_ids() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '0.17.4\n'
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"authenticated":true,"user_email":"qa@example.test","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"]}\n'
  exit 0
fi
if [ "$1" = "apps" ] && [ "$2" = "list" ]; then
  printf '{"items":[{"id":"raw-app-id-123","slug":"testnextjs"}]}\n'
  exit 0
fi
printf '{}\n'
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    std::fs::write(
        repo.join("axhub.yaml"),
        "name: testnextjs\nslug: testnextjs\n",
    )
    .unwrap();
    std::fs::write(repo.join(".gitignore"), "node_modules\n").unwrap();
    init_git_with_commit(&repo);

    let output = Command::new(bin())
        .args(["deploy-preview-summary", "--user-utterance", "배포해줘"])
        .env("AXHUB_BIN", &axhub)
        .env("HOME", temp.path())
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .current_dir(&repo)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("다음을 실행할게요:"), "{stdout}");
    assert!(stdout.contains("- 앱: testnextjs"), "{stdout}");
    assert!(stdout.contains("- 환경: production"), "{stdout}");
    assert!(stdout.contains("- 커밋:"), "{stdout}");
    assert!(stdout.contains("init: test"), "{stdout}");
    assert!(stdout.contains("진행할까요?"), "{stdout}");
    assert!(!stdout.contains("raw-app-id-123"), "{stdout}");
    assert!(!stdout.contains("qa@example.test"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("deploy-prep"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn deploy_approved_run_outputs_human_result_without_internal_consent_terms() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '0.17.4\n'
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"authenticated":true,"user_email":"qa@example.test","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"]}\n'
  exit 0
fi
if [ "$1" = "apps" ] && [ "$2" = "list" ]; then
  printf '{"items":[{"id":"raw-app-id-123","slug":"testnextjs"}]}\n'
  exit 0
fi
if [ "$1" = "deploy" ] && [ "$2" = "create" ]; then
  printf '{"event":"deploy_trigger_start","app":"raw-app-id-123"}\n'
  cat <<'JSON'
{
  "schema_version": "1",
  "status": "ok",
  "data": {
    "id": "raw-deploy-id-456",
    "app_slug": "testnextjs",
    "status": "queued"
  }
}
JSON
  exit 0
fi
if [ "$1" = "deploy" ] && [ "$2" = "status" ]; then
  printf '{"status":"succeeded","url":"https://testnextjs.test.axhub.ai","completed_at":"2099-01-01T00:00:30Z"}\n'
  exit 0
fi
printf '{}\n'
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    std::fs::write(
        repo.join("axhub.yaml"),
        "name: testnextjs\nslug: testnextjs\n",
    )
    .unwrap();
    std::fs::write(repo.join(".gitignore"), "node_modules\n").unwrap();
    init_git_with_commit(&repo);

    let output = Command::new(bin())
        .args(["deploy-approved-run", "--user-utterance", "응 진행해줘"])
        .env("AXHUB_BIN", &axhub)
        .env("HOME", temp.path())
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .current_dir(&repo)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("배포를 시작했어요"), "{stdout}");
    assert!(stdout.contains("배포가 완료됐어요"), "{stdout}");
    assert!(stdout.contains("- 앱: testnextjs"), "{stdout}");
    assert!(stdout.contains("- 커밋:"), "{stdout}");
    assert!(
        stdout.contains("https://testnextjs.test.axhub.ai"),
        "{stdout}"
    );
    assert!(!stdout.contains("raw-app-id-123"), "{stdout}");
    assert!(!stdout.contains("raw-deploy-id-456"), "{stdout}");
    assert!(!stdout.contains("qa@example.test"), "{stdout}");
    assert!(!stdout.contains("HMAC"), "{stdout}");
    assert!(!stdout.contains("consent"), "{stdout}");
    assert!(!stdout.contains("token"), "{stdout}");
    assert!(!stdout.contains("/axhub:deploy"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("deploy-prep"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn deploy_approved_run_reports_remote_commit_failure_without_recovery_trigger() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '0.17.4\n'
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"authenticated":true,"user_email":"qa@example.test","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"]}\n'
  exit 0
fi
if [ "$1" = "apps" ] && [ "$2" = "list" ]; then
  printf '{"items":[{"id":"raw-app-id-123","slug":"testnextjs"}]}\n'
  exit 0
fi
if [ "$1" = "deploy" ] && [ "$2" = "create" ]; then
  printf '{"event":"deploy_trigger_complete","deployment_id":"raw-deploy-id-456","status":"pending"}\n'
  exit 0
fi
if [ "$1" = "deploy" ] && [ "$2" = "status" ]; then
  cat <<'JSON'
{
  "schema_version": "1",
  "status": "ok",
  "data": {
    "id": "raw-deploy-id-456",
    "app_id": "raw-app-id-123",
    "commit_sha": "8b6444052f6072398c45f9379a788deed4f50d9b",
    "status": "failed",
    "failure_reason": {
      "category": "configuration",
      "code": "resolve.commit_not_found",
      "message": "커밋을 찾을 수 없어요",
      "stage": "resolve"
    }
  }
}
JSON
  exit 0
fi
printf '{}\n'
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    std::fs::write(
        repo.join("axhub.yaml"),
        "name: testnextjs\nslug: testnextjs\n",
    )
    .unwrap();
    std::fs::write(repo.join(".gitignore"), "node_modules\n").unwrap();
    init_git_with_commit(&repo);

    let output = Command::new(bin())
        .args(["deploy-approved-run", "--user-utterance", "응 진행해줘"])
        .env("AXHUB_BIN", &axhub)
        .env("HOME", temp.path())
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .current_dir(&repo)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("배포를 시작했어요"), "{stdout}");
    assert!(
        stdout.contains("배포가 끝났지만 성공 상태는 아니에요"),
        "{stdout}"
    );
    assert!(
        stdout.contains("커밋을 원격 저장소에서 찾을 수 없어요"),
        "{stdout}"
    );
    assert!(!stdout.contains("resolve.commit_not_found"), "{stdout}");
    assert!(!stdout.contains("raw-app-id-123"), "{stdout}");
    assert!(!stdout.contains("raw-deploy-id-456"), "{stdout}");
    assert!(!stdout.contains("qa@example.test"), "{stdout}");
    assert!(!stdout.contains("배포 번호"), "{stdout}");
    assert!(!stdout.contains("명령"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("deploy-prep"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn inspect_config_summary_humanizes_raw_cli_json() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "manifest" ] && [ "$2" = "validate" ]; then
  cat <<'JSON'
{"schema_version":"1","status":"ok","data":{"app":null,"ci":null,"deploy":{"commands":[],"method":"docker","port":null}}}
JSON
  exit 0
fi
if [ "$1" = "config" ] && [ "$2" = "explain" ]; then
  cat <<'JSON'
{"active_profile":"default","agent_mode":true,"endpoint":"https://api.axhub.ai","token":{"present":false,"source":"keychain"}}
JSON
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("inspect-config-summary")
        .env("AXHUB_BIN", &axhub)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("매니페스트와 설정을 확인했어요."));
    assert!(stdout.contains("매니페스트 문법은 맞지만 실제 배포에 필요한 항목이 아직 비어 있어요."));
    assert!(stdout.contains("로그인 정보가 없어 배포 전에 다시 로그인 확인이 필요해요."));
    assert!(!stdout.contains("app: null"), "{stdout}");
    assert!(!stdout.contains("token"), "{stdout}");
    assert!(!stdout.contains("[]"), "{stdout}");
    assert!(!stdout.contains("https://api.axhub.ai"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn status_summary_resolves_manifest_and_humanizes_deploy_status() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    std::fs::write(
        temp.path().join(".git_marker"),
        "unused but keeps temp populated\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub-status");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5" = "--json deploy list --app testnextjs" ]; then
  echo '{"items":[{"id":"dep_1","app_id":"app_1","status":"succeeded","commit_sha":"abcdef123456","commit_message":"ship test","branch":"main","created_at":"2026-06-04T01:00:00Z"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5 $6" = "--json deploy status dep_1 --app testnextjs" ]; then
  echo '{"id":"dep_1","app_id":"app_1","status":"succeeded","started_at":"2026-06-04T01:00:00Z","completed_at":"2026-06-04T01:02:03Z","failure_reason":null}'
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("status-summary")
        .arg("--user-utterance")
        .arg("지금 진행 중인 배포 어디까지 됐어?")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("배포 상태를 확인했어요."));
    assert!(stdout.contains("앱 testnextjs의 최근 배포는 성공했어요."));
    assert!(stdout.contains("커밋: abcdef1"));
    assert!(stdout.contains("배포는 끝난 상태예요."));
    assert!(!stdout.contains("APP 미해석"), "{stdout}");
    assert!(!stdout.contains("items[0]"), "{stdout}");
    assert!(!stdout.contains("headless"), "{stdout}");
    assert!(!stdout.contains("deploy list"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn logs_summary_resolves_latest_deploy_and_humanizes_logs() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5" = "--json deploy list --app testnextjs" ]; then
  echo '{"items":[{"id":"dep_1","app_id":"app_1","status":"succeeded","commit_sha":"abcdef123456","commit_message":"ship test","branch":"main","created_at":"2026-06-04T01:00:00Z"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5 $6 $7 $8" = "--json deploy logs dep_1 --app testnextjs --limit 50" ]; then
  echo '{"type":"log","message":"INFO server started"}'
  echo '{"type":"log","message":"ERROR missing API key axhub_pat_abcdefghijklmnopqrstuvwxyz"}'
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("logs-summary")
        .arg("--user-utterance")
        .arg("로그 좀 보여줘")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("로그를 확인했어요."));
    assert!(stdout.contains("앱 testnextjs의 최근 배포는 성공했어요."));
    assert!(stdout.contains("커밋: abcdef1"));
    assert!(stdout.contains("최근 로그:"));
    assert!(stdout.contains("INFO server started"));
    assert!(stdout.contains("눈에 띄는 오류: ERROR missing API key"));
    assert!(
        !stdout.contains("axhub_pat_abcdefghijklmnopqrstuvwxyz"),
        "{stdout}"
    );
    assert!(!stdout.contains("Resolve deployment first"), "{stdout}");
    assert!(!stdout.contains("No deploy id cached"), "{stdout}");
    assert!(!stdout.contains("List deployments for app"), "{stdout}");
    assert!(!stdout.contains("deploy logs"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn open_summary_resolves_current_app_and_humanizes_url() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1 $2 $3" = "open testnextjs --json" ]; then
  echo '{"schema_version":"1","status":"ok","data":{"opened":true,"status":"opening","url":"https://app.axhub.ai/apps/testnextjs"}}'
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("open-summary")
        .arg("--user-utterance")
        .arg("라이브 페이지 열어봐")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("앱 페이지를 확인했어요."));
    assert!(stdout.contains("앱: testnextjs"));
    assert!(stdout.contains("URL: https://app.axhub.ai/apps/testnextjs"));
    assert!(stdout.contains("브라우저에서 열기 요청을 보냈어요."));
    assert!(!stdout.contains("Read QA result files"), "{stdout}");
    assert!(!stdout.contains("ToolSearch"), "{stdout}");
    assert!(!stdout.contains("Chrome MCP"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn verify_summary_resolves_current_app_and_humanizes_live_verdict() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5" = "--json deploy list --app testnextjs" ]; then
  echo '{"items":[{"id":"dep_1","app_id":"app_1","status":"succeeded","commit_sha":"abcdef123456","created_at":"2026-06-04T01:00:00Z"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5 $6" = "--json deploy status dep_1 --app testnextjs" ]; then
  echo '{"state":"succeeded","last_deploy_id":"dep_1","last_deploy_age_secs":42}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5 $6 $7 $8" = "--json deploy logs dep_1 --app testnextjs --limit 50" ]; then
  echo 'INFO server started'
  echo 'INFO ready'
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("verify-summary")
        .arg("--user-utterance")
        .arg("방금 배포 진짜 열리는지 확인해줘")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("배포 검증을 완료했어요."));
    assert!(stdout.contains("✅ 라이브 확정"));
    assert!(stdout.contains("앱 testnextjs는 최근 배포 기준으로 열리는 상태예요."));
    assert!(stdout.contains("ERROR/FATAL은 보이지 않았어요."));
    assert!(!stdout.contains("u@example.com"), "{stdout}");
    assert!(!stdout.contains("id="), "{stdout}");
    assert!(!stdout.contains("deploy="), "{stdout}");
    assert!(!stdout.contains("status="), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("axhub verify"), "{stdout}");
    assert!(!stdout.contains("stale"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn trace_summary_reports_no_recent_failure_without_internal_leaks() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5" = "--json deploy list --app testnextjs" ]; then
  echo '{"items":[{"id":"dep_success_1","app_id":"app_1","status":"succeeded","commit_sha":"abcdef123456","created_at":"2026-06-04T01:00:00Z"},{"id":"dep_success_0","app_id":"app_1","status":"succeeded","commit_sha":"fff111222333","created_at":"2026-06-03T01:00:00Z"}]}'
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("trace-summary")
        .arg("--user-utterance")
        .arg("배포 실패 원인 알려줘")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("배포 기록을 확인했어요."), "{stdout}");
    assert!(
        stdout.contains("최근 실패한 배포는 찾지 못했어요."),
        "{stdout}"
    );
    assert!(stdout.contains("최신 배포는 성공했어요."), "{stdout}");
    assert!(stdout.contains("최근 커밋: abcdef1"), "{stdout}");
    for forbidden in [
        "u@example.com",
        "dep_success",
        "succeeded",
        "failure_reason",
        "matched_patterns",
        "build_log_errors",
        "preflight",
        "/axhub:",
        "axhub:trace",
        "Run axhub",
        "Trace latest deploy",
        "deploy=",
        "status=",
    ] {
        assert!(
            !stdout.contains(forbidden),
            "{forbidden} leaked in {stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn env_summary_resolves_current_app_and_masks_values() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1" = "env" ] && [ "$2" = "list" ]; then
  cat <<'JSON'
{"items":[{"key":"TEST_KEY","stage":"runtime","secret":false,"value":"test_value_123"},{"key":"SECRET_KEY","stage":"runtime","secret":true,"value":"super_secret"}],"total":2}
JSON
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("env-summary")
        .arg("--user-utterance")
        .arg("환경변수 뭐 있어?")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("환경변수 목록을 확인했어요."));
    assert!(stdout.contains("총 2개"));
    assert!(stdout.contains("TEST_KEY"));
    assert!(stdout.contains("SECRET_KEY"));
    assert!(stdout.contains("있음(숨김)"));
    assert!(!stdout.contains("test_value_123"), "{stdout}");
    assert!(!stdout.contains("super_secret"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("axhub env"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn resources_summary_keeps_desktop_cleanup_flow_human_readable() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "resources list --json" ]; then
  echo '{"schema_version":"1","status":"ok","data":[]}'
  exit 0
fi
if [ "$1 $2 $3" = "connectors list --enabled-only" ]; then
  echo '{"schema_version":"1","status":"ok","data":[]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("resources-summary")
        .arg("--user-utterance")
        .arg("리소스 정리하고 싶어")
        .env("AXHUB_BIN", &axhub)
        .env_remove("AXHUB_PROFILE")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("리소스 현황을 확인했어요."));
    assert!(stdout.contains("데이터베이스 연결: 0개"));
    assert!(stdout.contains("정리할 리소스: 0개"));
    assert!(stdout.contains("어떤 정리를 할까요?"));
    assert!(stdout.contains("미리보기와 승인 후 진행할게요."));
    assert!(!stdout.contains("u@example.com"), "{stdout}");
    assert!(!stdout.contains("connector/resource"), "{stdout}");
    assert!(!stdout.contains("catalog kind"), "{stdout}");
    assert!(!stdout.contains("read-only"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("/axhub:"), "{stdout}");
    assert!(!stdout.contains("axhub resources"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn github_summary_checks_axhub_app_connection_without_local_git_remote() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("axhub.yaml"),
        "app:\n  slug: testnextjs\n  name: testnextjs\n",
    )
    .unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","scopes":["read","write"]}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_1","slug":"testnextjs"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4 $5 $6" = "apps git status --app testnextjs --json" ]; then
  echo '{"schema_version":"1","status":"ok","data":{"connected":true,"provider":"github","repo_full_name":"jocoding-ax-partners/testnextjs","branch":"main","installation_id":133340904}}'
  exit 0
fi
if [ "$1 $2" = "rev-parse --is-inside-work-tree" ]; then
  echo false
  exit 1
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("github-summary")
        .arg("--user-utterance")
        .arg("이 앱 깃허브랑 연결돼 있어?")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GitHub 연결 상태를 확인했어요."));
    assert!(stdout.contains("GitHub 저장소에 연결되어 있어요."));
    assert!(stdout.contains("저장소: jocoding-ax-partners/testnextjs"));
    assert!(stdout.contains("브랜치: main"));
    for forbidden in [
        "u@example.com",
        "installation_id",
        "133340904",
        "git remote",
        "remote.origin",
        "axhub apps git",
        "/axhub:",
        "preflight",
    ] {
        assert!(
            !stdout.contains(forbidden),
            "{forbidden} leaked in {stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn doctor_summary_masks_identity_and_avoids_internal_labels() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("doctor-summary")
        .arg("--user-utterance")
        .arg("axhub CLI 설치 상태 괜찮아?")
        .env("AXHUB_BIN", &axhub)
        .env_remove("AXHUB_PROFILE")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("설치 상태를 확인했어요."));
    assert!(stdout.contains("CLI: v0.17.4, 플러그인과 호환돼요."));
    assert!(stdout.contains("로그인: 되어 있어요"));
    assert!(stdout.contains("다시 로그인할 필요 없어요"));
    assert!(stdout.contains("권한: read, write"));
    assert!(stdout.contains("프로필: default"));
    assert!(!stdout.contains("u@example.com"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("axhub:doctor"), "{stdout}");
    assert!(!stdout.contains("/axhub:"), "{stdout}");
    assert!(!stdout.contains("auth status"), "{stdout}");
    assert!(
        !stdout.contains(temp.path().to_string_lossy().as_ref()),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn install_summary_stops_when_cli_already_exists() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("install-summary")
        .arg("--user-utterance")
        .arg("axhub CLI 설치해줘")
        .env("AXHUB_BIN", &axhub)
        .env_remove("AXHUB_PROFILE")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("설치 상태를 확인했어요."));
    assert!(stdout.contains("axhub CLI: 이미 설치되어 있어요. (v0.17.4)"));
    assert!(stdout.contains("설치 작업: 지금은 필요 없어요."));
    for forbidden in [
        "u@example.com",
        "auth status",
        "preflight",
        "프로필",
        "권한:",
        "/axhub:",
        "curl ",
        "install.sh",
    ] {
        assert!(
            !stdout.contains(forbidden),
            "{forbidden} leaked in {stdout}"
        );
    }
    assert!(
        !stdout.contains(temp.path().to_string_lossy().as_ref()),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn update_summary_renders_korean_check_without_raw_fields() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1 $2 $3" = "update check --json" ]; then
  echo '{"current":"v0.17.4","latest":"v0.17.5","has_update":true}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("update-summary")
        .arg("--user-utterance")
        .arg("업데이트 필요한지 봐줘")
        .env("AXHUB_BIN", &axhub)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("업데이트를 확인했어요."));
    assert!(stdout.contains("현재 버전: v0.17.4"));
    assert!(stdout.contains("새 버전: v0.17.5"));
    assert!(stdout.contains("업데이트: 받을 수 있어요."));
    assert!(stdout.contains("적용: 아직 시작하지 않았어요."));
    assert!(stdout.contains("미리보기와 승인을 받을게요."));
    for forbidden in [
        "has_update",
        "update check",
        "axhub update",
        "raw",
        "/axhub:",
        "preflight",
    ] {
        assert!(
            !stdout.contains(forbidden),
            "{forbidden} leaked in {stdout}"
        );
    }
    assert!(
        !stdout.contains(temp.path().to_string_lossy().as_ref()),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn rollback_summary_explains_noop_without_raw_deploy_or_commit_leaks() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("axhub.yaml"), "slug: rollbackapp\n").unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"],"current_app":"rollbackapp"}'
  exit 0
fi
if [ "$1 $2 $3" = "apps list --json" ]; then
  echo '{"items":[{"id":"app_rollback","slug":"rollbackapp","name":"Rollback App"}]}'
  exit 0
fi
if [ "$1 $2 $3 $4" = "--json deploy list --app" ]; then
  echo '{"items":[{"id":"dep_failed_123","app_slug":"rollbackapp","status":"failed","commit_sha":"abcdef1234567890","commit_message":"commit_not_found","created_at":"2026-06-04T01:02:03Z"},{"id":"dep_success_456","app_slug":"rollbackapp","status":"succeeded","commit_sha":"1234567890abcdef","commit_message":"stable","created_at":"2026-06-03T01:02:03Z"}]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("rollback-summary")
        .arg("--user-utterance")
        .arg("방금 배포 되돌려줘")
        .env("AXHUB_BIN", &axhub)
        .env_remove("AXHUB_PROFILE")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("되돌릴 수 있는 배포를 확인했어요."));
    assert!(stdout.contains("방금 시도한 배포는 공개 버전으로 반영되지 않았어요."));
    assert!(stdout.contains("현재 공개된 버전은 이미 최근 성공 버전으로 보입니다."));
    assert!(stdout.contains("더 이전에 되돌릴 성공 배포는 찾지 못했어요."));
    assert!(stdout.contains("지금은 그대로 두는 게 안전해요."));
    for forbidden in [
        "u@example.com",
        "rollbackapp",
        "dep_failed",
        "dep_success",
        "abcdef",
        "1234567",
        "failed",
        "succeeded",
        "commit_not_found",
        "no-op",
        "preflight",
        "axhub rollback",
        "/axhub:",
    ] {
        assert!(
            !stdout.contains(forbidden),
            "{forbidden} leaked in {stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn auth_summary_answers_login_need_without_identity_leaks() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "0.17.4"
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{"user_email":"u@example.com","expires_at":"2099-01-01T00:00:00Z","scopes":["read","write"],"profile":"default","tenants":[{"tenant_slug":"acme"}]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&axhub).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&axhub, permissions).unwrap();

    let output = Command::new(bin())
        .arg("auth-summary")
        .arg("--user-utterance")
        .arg("나 로그인 돼 있어?")
        .env("AXHUB_BIN", &axhub)
        .env_remove("AXHUB_PROFILE")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("로그인 상태를 확인했어요."));
    assert!(stdout.contains("로그인: 되어 있어요."));
    assert!(stdout.contains("다시 로그인: 지금은 필요 없어요."));
    assert!(!stdout.contains("u@example.com"), "{stdout}");
    assert!(!stdout.contains("read"), "{stdout}");
    assert!(!stdout.contains("write"), "{stdout}");
    assert!(!stdout.contains("default"), "{stdout}");
    assert!(!stdout.contains("acme"), "{stdout}");
    assert!(!stdout.contains("2099"), "{stdout}");
    assert!(!stdout.contains("auth status"), "{stdout}");
    assert!(!stdout.contains("preflight"), "{stdout}");
    assert!(!stdout.contains("/axhub:"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn statusline_summary_preserves_existing_statusbar_without_internal_leaks() {
    let home = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let claude_dir = home.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    let settings = claude_dir.join("settings.json");
    let existing_command =
        "sh ${CLAUDE_CONFIG_DIR:-$HOME/.claude}/hud/omc-hud-cache.sh --theme calm";
    let existing_settings = serde_json::json!({
        "statusLine": {
            "type": "command",
            "command": existing_command
        }
    });
    std::fs::write(
        &settings,
        serde_json::to_string_pretty(&existing_settings).unwrap(),
    )
    .unwrap();

    let output = Command::new(bin())
        .arg("statusline-summary")
        .arg("--user-utterance")
        .arg("상태바 켜줘")
        .env("HOME", home.path())
        .env("XDG_STATE_HOME", state.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(stdout.contains("상태바 설정을 확인했어요."), "{stdout}");
    assert!(
        stdout.contains("이미 다른 상태바가 켜져 있어요."),
        "{stdout}"
    );
    assert!(stdout.contains("덮어쓰지 않았어요."), "{stdout}");

    for forbidden in [
        existing_command,
        "omc-hud-cache",
        "CLAUDE_CONFIG_DIR",
        "$HOME",
        "settings.json",
        "statusLine",
        "settings-merge",
        "autowire-statusline",
        "axhub-helpers",
        "Exit",
        "exit",
        "--scope",
        "wire",
        "PreservedOther",
    ] {
        assert!(
            !combined.contains(forbidden),
            "leaked {forbidden:?} in {combined}"
        );
    }

    let stored = std::fs::read_to_string(settings).unwrap();
    assert!(
        stored.contains(existing_command),
        "existing status bar command must be preserved"
    );
}

// Approach E (Phase 2): no skill path enforcement for generic prompts.
#[cfg(unix)]
#[test]
fn cli_prompt_route_no_forced_skills_context() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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

// Approach E (Phase 2): generic prompts produce identical preflight-only output
// (no generic intent classification by Rust router).
#[cfg(unix)]
#[test]
fn cli_prompt_route_no_intent_routing() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = temp.path().join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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

    // Compare the AGENT-FACING channel only (`hookSpecificOutput` →
    // additionalContext). The Rust router stays preflight-only for generic
    // prompts, with narrow UltraQA-proven hints for high-risk axhub surfaces.
    // The orthogonal top-level
    // `systemMessage` channel CAN differ by intent — the once-per-project
    // deploy-migration grace nudge (spec 006 §43, AC 11) rides it for an
    // authed/no-marker/implicit-deploy prompt — and is deliberately excluded
    // here so this assertion is robust to the runner's ambient auth state.
    let snapshot = |prompt: &str| -> String {
        let input =
            serde_json::json!({"hook_event_name":"UserPromptSubmit","prompt":prompt}).to_string();
        let output = run_stdin(
            &["prompt-route"],
            &input,
            &[("AXHUB_BIN", axhub.to_str().unwrap())],
        );
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(stdout.trim()).expect("hook JSON");
        json.get("hookSpecificOutput")
            .expect("hookSpecificOutput present")
            .to_string()
    };

    // Generic non-axhub prompts → identical agent-facing context.
    let a = snapshot("배포해줘");
    let b = snapshot("문장 다듬어줘");
    let c = snapshot("아무말 대잔치");
    assert_eq!(b, c);
    assert!(a.contains("배포 준비를 확인할게요"), "{a}");
    assert!(a.contains("AXHub live deployment request"), "{a}");
    assert!(a.contains("deploy-preview-summary"), "{a}");
    assert!(a.contains("Do not write route labels"), "{a}");
    assert!(!a.contains("AXHub deploy workflow"), "{a}");
    assert!(!b.contains("Skill(axhub:"), "{b}");
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
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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
    assert!(session_stdout.contains("처음 설정 도와줘"));
    assert!(session_stdout.contains("말씀해주세요"));
    assert!(session_stdout.contains("감사 로그"));
    assert!(session_stdout.contains("안전한 기본값"));
    assert!(!session_stdout.contains("/axhub:"), "{session_stdout}");
    assert!(!session_stdout.contains("SKILL"), "{session_stdout}");
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
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
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
fn read_audit_records(state: &std::path::Path) -> Vec<serde_json::Value> {
    let dir = audit_dir_path(state);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("routing-audit-") || !name.ends_with(".jsonl") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    out.push(v);
                }
            }
        }
    }
    out
}

// AC-12: prompt-route writes the shared routing decision type per prompt into
// routing-audit-*.jsonl. Prompts chosen to hit deterministic priority rules
// (0/b/c) so the decision is independent of cwd-marker and token-file state.
#[cfg(unix)]
#[test]
fn cli_prompt_route_audit_records_decision_type() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    invoke_prompt_route("/deploy", &axhub, &state_s); // rule 0 slash → explicit
    invoke_prompt_route("vercel 로 배포해", &axhub, &state_s); // rule c foreign → yield
    invoke_prompt_route("axhub 로 배포해", &axhub, &state_s); // rule b keyword → axhub

    let records = read_audit_records(&state);
    assert_eq!(records.len(), 3, "expected 3 audit lines, got {records:?}");

    let by_decision = |d: &str| records.iter().find(|r| r["decision"] == d);

    let explicit = by_decision("explicit").expect("explicit record");
    assert_eq!(explicit["explicit_invocation"], serde_json::json!(true));

    let yield_rec = by_decision("yield").expect("yield record");
    assert_eq!(
        yield_rec["foreign_keyword_present"],
        serde_json::json!(true)
    );
    assert_eq!(yield_rec["explicit_invocation"], serde_json::json!(false));

    let axhub_rec = by_decision("axhub").expect("axhub record");
    assert_eq!(axhub_rec["axhub_keyword_present"], serde_json::json!(true));

    // Each line carries the decision enum + the four decide() inputs.
    for r in &records {
        assert!(r["decision"].is_string(), "decision missing: {r}");
        assert!(
            r["marker_present"].is_boolean(),
            "marker_present missing: {r}"
        );
        assert!(r["authed"].is_boolean(), "authed missing: {r}");
        assert!(
            r["explicit_invocation"].is_boolean(),
            "explicit_invocation missing: {r}"
        );
    }
}

// AC-12: routing-stats reads & reports the decision breakdown + ignore rate.
#[cfg(unix)]
#[test]
fn cli_routing_stats_reports_decision_breakdown() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(&temp);
    let state = temp.path().join("state");
    let state_s = state.display().to_string();

    invoke_prompt_route("vercel 로 배포해", &axhub, &state_s); // yield
    invoke_prompt_route("axhub 로 배포해", &axhub, &state_s); // axhub

    let stats = run_stdin(
        &["routing-stats", "--json"],
        "",
        &[("XDG_STATE_HOME", state_s.as_str())],
    );
    assert_eq!(stats.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&stats.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert!(
        parsed.get("decision_counts").is_some(),
        "decision_counts missing: {stdout}"
    );
    assert_eq!(parsed["decision_counts"]["yield"], serde_json::json!(1));
    assert_eq!(parsed["decision_counts"]["axhub"], serde_json::json!(1));
    assert!(parsed.get("ignore_rate").is_some(), "ignore_rate missing");
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
    assert!(msg.contains("처음 설정 도와줘"), "{msg}");
    assert!(msg.contains("도움말 보여줘"), "{msg}");
    assert!(msg.contains("설치 상태 확인해줘"), "{msg}");
    assert!(!msg.contains("/axhub:"), "{msg}");
    assert!(!msg.contains("SKILL"), "{msg}");
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
    let state = temp.path().join("state").display().to_string();
    let runtime = temp.path().join("runtime").display().to_string();
    let envs = [
        ("XDG_STATE_HOME", state.as_str()),
        ("XDG_RUNTIME_DIR", runtime.as_str()),
        ("AXHUB_PROFILE", "prod"),
        ("CLAUDE_SESSION_ID", "bootstrap-profile-session"),
    ];
    let output = run_in_dir_env(&["bootstrap", "--auto-chain", "--json"], temp.path(), &envs);
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["state"], "apps_create_pending");
    assert_eq!(json["next_action"], "apps_create");
    // v0.17.3 `apps create --from-file` is JSON-only, so first-run app creation
    // uses the `--name`/`--slug` lane (slug parsed from the manifest).
    assert_eq!(json["command"][0], "axhub");
    assert_eq!(json["command"][1], "apps");
    assert_eq!(json["command"][2], "create");
    assert_eq!(json["command"][3], "--name");
    assert_eq!(json["command"][4], "paydrop");
    assert_eq!(json["command"][5], "--slug");
    assert_eq!(json["command"][6], "paydrop");
    assert_eq!(json["command"][7], "--json");
    assert_eq!(json["command"][8], "--profile");
    assert_eq!(json["command"][9], "prod");
    assert!(json.get("consent_binding").is_none());
    assert!(json.get("binding_hash").is_none());
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

    let replayed = run_in_dir_env(&["bootstrap", "--auto-chain", "--json"], temp.path(), &envs);
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
    assert_eq!(replayed_json["command"], json["command"]);
    assert!(replayed_json.get("consent_binding").is_none());
    assert!(replayed_json.get("binding_hash").is_none());
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
    assert!(event_names.contains(&"remote_action_planned_by_helper"));

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

    let remote_action = events
        .iter()
        .find(|event| event["event"] == "remote_action_planned_by_helper")
        .unwrap();
    assert_eq!(remote_action["record_event"], "apps_create");
    assert_eq!(remote_action["decision_class"], "remote_destructive_plan");
    assert_eq!(
        remote_action["retry_policy"],
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
    assert_eq!(deploy_plan_json["state"], "deploy_create_pending");
    assert!(deploy_plan_json.get("consent_binding").is_none());

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
    assert_eq!(
        json["sdk_conversion"]["schema_version"],
        "sdk-conversion/v1"
    );
    let sdk_candidates = json["sdk_conversion"]["candidates"].as_array().unwrap();
    let node_candidate = sdk_candidates
        .iter()
        .find(|candidate| candidate["path"] == ".")
        .unwrap();
    assert_eq!(node_candidate["language"], "node");
    assert_eq!(node_candidate["dependency_hint"], "npm install @ax-hub/sdk");
    assert_eq!(node_candidate["wrapper_target_path"], "src/axhub.ts");
    assert!(node_candidate["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("import { AxHubClient } from '@ax-hub/sdk'"));
    assert_eq!(node_candidate["risk"], "high");
    assert!(node_candidate["hard_stop_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason
            .as_str()
            .unwrap()
            .contains("검증 명령 또는 테스트 anchor")));
    // devex-D1=C: per-reason override policy ships alongside the reasons, and
    // plan_only is exposed for the SKILL's structural plan-only gate.
    let policy = node_candidate["hard_stop_policy"].as_array().unwrap();
    assert!(!policy.is_empty());
    let missing_verification = policy
        .iter()
        .find(|entry| entry["code"] == "missing_verification")
        .expect("missing_verification policy present");
    assert_eq!(missing_verification["overridable"], true);
    assert!(node_candidate["plan_only"].is_boolean());
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
    let sdk_candidates = json["sdk_conversion"]["candidates"].as_array().unwrap();
    let web_sdk = sdk_candidates
        .iter()
        .find(|candidate| candidate["path"] == "apps/web")
        .unwrap();
    assert_eq!(web_sdk["language"], "node");
    assert!(web_sdk["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("import { AxHubClient } from '@ax-hub/sdk'"));
    let api_sdk = sdk_candidates
        .iter()
        .find(|candidate| candidate["path"] == "services/api")
        .unwrap();
    assert_eq!(api_sdk["language"], "go");
    assert_eq!(
        api_sdk["dependency_hint"],
        "go get github.com/jocoding-ax-partners/axhub-sdk-go"
    );
    assert_eq!(
        api_sdk["wrapper_target_path"],
        "services/api/axhub_client.go"
    );
    assert!(api_sdk["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("NewAxHubClient"));

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

#[test]
fn cli_migrate_plan_persist_planning_writes_spec_only_pending_approval_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let web = temp.path().join("apps/web");
    let api = temp.path().join("services/api");
    std::fs::create_dir_all(web.join("src")).unwrap();
    std::fs::create_dir_all(web.join("tests")).unwrap();
    std::fs::create_dir_all(&api).unwrap();
    std::fs::write(
        web.join("package.json"),
        r#"{"dependencies":{"vite":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(web.join("src/web.ts"), "export const web = true;").unwrap();
    std::fs::write(web.join("tests/app.test.ts"), "expect(true).toBe(true);").unwrap();
    std::fs::write(api.join("go.mod"), "module example.com/api\n").unwrap();

    let output = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--app-path",
        "apps/web",
        "--persist-planning",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["planning"]["mode"], "spec_only");
    assert_eq!(
        json["planning_persistence"]["reason"],
        "spec_only_pending_approval_written"
    );
    assert_eq!(
        json["planning_persistence"]["run_state"],
        "pending_approval"
    );
    assert!(json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        // normalize separators: the path is OS-native (backslashes on Windows)
        .replace('\\', "/")
        .contains(".axhub/plan/runs/"));
}

#[test]
fn cli_migrate_plan_persist_planning_writes_full_consensus_discover_scaffold() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/db.ts"),
        "export async function rows(db: any) { return db.query('select * from users'); }",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/auth.ts"),
        "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
    )
    .unwrap();

    let output = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--persist-planning",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["planning"]["mode"], "full_consensus");
    assert_eq!(
        json["planning_persistence"]["reason"],
        "full_consensus_scaffold_written"
    );
    assert_eq!(json["planning_persistence"]["run_state"], "running");
    assert_eq!(
        json["planning_persistence"]["approval_state"],
        "needs_revision"
    );
    assert!(json["planning_persistence"]["paths"]["discover_markdown"]
        .as_str()
        .unwrap()
        .contains("01-discover.md"));
}

#[test]
fn cli_migrate_stage_write_advances_full_consensus_to_pending_approval() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/db.ts"),
        "export async function rows(db: any) { return db.query('select * from users'); }",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/auth.ts"),
        "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
    )
    .unwrap();

    let seeded = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--persist-planning",
        "--json",
    ]);
    assert_eq!(seeded.status.code(), Some(0));
    let seeded_json: serde_json::Value = serde_json::from_slice(&seeded.stdout).unwrap();
    let run_json = seeded_json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        .to_string();

    let planner_md = temp.path().join("planner.md");
    let architect_md = temp.path().join("architect.md");
    let critic_md = temp.path().join("critic.md");
    let reviewer_md = temp.path().join("reviewer.md");
    let adr_md = temp.path().join("adr.md");
    std::fs::write(&planner_md, "# Planner\n\n- plan").unwrap();
    std::fs::write(&architect_md, "# Architect\n\n- clear").unwrap();
    std::fs::write(&critic_md, "# Critic\n\n- okay").unwrap();
    std::fs::write(&reviewer_md, "# Reviewer\n\n- approve").unwrap();
    std::fs::write(&adr_md, "# ADR\n\n- chosen").unwrap();

    for (stage, file) in [
        ("planner", &planner_md),
        ("architect", &architect_md),
        ("critic", &critic_md),
        ("adr", &adr_md),
    ] {
        let output = run(&[
            "migrate-stage-write",
            "--run-json",
            &run_json,
            "--stage",
            stage,
            "--markdown-file",
            file.to_str().unwrap(),
            "--json",
        ]);
        assert_eq!(output.status.code(), Some(0));
    }

    let reviewer_output = run(&[
        "migrate-stage-write",
        "--run-json",
        &run_json,
        "--stage",
        "reviewer",
        "--markdown-file",
        reviewer_md.to_str().unwrap(),
        "--run-state",
        "pending_approval",
        "--approval-state",
        "pending_approval",
        "--json",
    ]);
    assert_eq!(reviewer_output.status.code(), Some(0));
    let reviewer_json: serde_json::Value = serde_json::from_slice(&reviewer_output.stdout).unwrap();
    assert_eq!(reviewer_json["run_state"], "pending_approval");
    assert_eq!(reviewer_json["approval_state"], "pending_approval");

    let run_state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_json).unwrap()).unwrap();
    assert_eq!(run_state["state"], "pending_approval");
    let approval_path = std::path::Path::new(&run_json)
        .parent()
        .unwrap()
        .join("approval.json");
    let approval: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&approval_path).unwrap()).unwrap();
    assert_eq!(approval["state"], "pending_approval");
    assert!(
        approval["approved_stage_artifacts"]
            .as_array()
            .unwrap()
            .len()
            >= 4
    );
    assert!(approval["adr_sha256"].as_str().is_some());
}

#[test]
fn cli_migrate_stage_write_supports_revision_loop() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/db.ts"),
        "export async function rows(db: any) { return db.query('select * from users'); }",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/auth.ts"),
        "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
    )
    .unwrap();

    let seeded = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--persist-planning",
        "--json",
    ]);
    let seeded_json: serde_json::Value = serde_json::from_slice(&seeded.stdout).unwrap();
    let run_json = seeded_json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        .to_string();

    let planner_md = temp.path().join("planner-r1.md");
    let architect_md = temp.path().join("architect-r1.md");
    let critic_md = temp.path().join("critic-r1.md");
    let planner_r2_md = temp.path().join("planner-r2.md");
    std::fs::write(&planner_md, "# Planner r1").unwrap();
    std::fs::write(&architect_md, "# Architect r1").unwrap();
    std::fs::write(&critic_md, "# Critic r1\n\n- revise").unwrap();
    std::fs::write(&planner_r2_md, "# Planner r2").unwrap();

    for (stage, file, run_state) in [
        ("planner", &planner_md, None),
        ("architect", &architect_md, None),
        ("critic", &critic_md, Some("needs_revision")),
        ("planner", &planner_r2_md, Some("running")),
    ] {
        let mut argv = vec![
            "migrate-stage-write",
            "--run-json",
            &run_json,
            "--stage",
            stage,
            "--markdown-file",
            file.to_str().unwrap(),
            "--json",
        ];
        if let Some(state) = run_state {
            argv.extend(["--run-state", state]);
        }
        let output = run(&argv);
        assert_eq!(output.status.code(), Some(0));
    }

    let run_state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_json).unwrap()).unwrap();
    assert_eq!(run_state["state"], "running");
    let stages_dir = std::path::Path::new(&run_json)
        .parent()
        .unwrap()
        .join("stages");
    let entries = std::fs::read_dir(stages_dir).unwrap().count();
    assert!(
        entries >= 10,
        "expected discover + planner/architect/critic/planner markdown+meta files"
    );
}

#[test]
fn cli_migrate_wave_plan_accepts_same_app_and_falls_back_on_conflict() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/db.ts"),
        "export async function rows(db: any) { return db.query('select * from users'); }",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/auth.ts"),
        "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
    )
    .unwrap();

    let seeded = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--persist-planning",
        "--json",
    ]);
    let seeded_json: serde_json::Value = serde_json::from_slice(&seeded.stdout).unwrap();
    let run_json = seeded_json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        .to_string();
    let app_key = seeded_json["planning"]["app_key"]
        .as_str()
        .unwrap()
        .to_string();

    let first = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "review-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--write-target",
        "stages/05-reviewer-a.md",
        "--independence-proof",
        "disjoint evidence set",
        "--json",
    ]);
    assert_eq!(first.status.code(), Some(0));
    let first_json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first_json["serial_fallback"], false);
    assert_eq!(first_json["parallelism_enabled"], true);

    let second = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "review-b",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--write-target",
        "stages/05-reviewer-a.md",
        "--independence-proof",
        "conflicting target",
        "--json",
    ]);
    assert_eq!(second.status.code(), Some(0));
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second_json["serial_fallback"], true);
    assert_eq!(second_json["parallelism_enabled"], false);
    assert!(second_json["fallback_reason"]
        .as_str()
        .unwrap()
        .contains("write target"));

    let run_state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_json).unwrap()).unwrap();
    assert_eq!(run_state["parallelism"]["enabled"], false);
    assert!(run_state["parallelism"]["fallback_reason"]
        .as_str()
        .is_some());
}

#[test]
fn cli_migrate_approve_promotes_spec_only_latest_pointer() {
    let temp = tempfile::tempdir().unwrap();
    let web = temp.path().join("apps/web");
    let api = temp.path().join("services/api");
    std::fs::create_dir_all(web.join("src")).unwrap();
    std::fs::create_dir_all(web.join("tests")).unwrap();
    std::fs::create_dir_all(&api).unwrap();
    std::fs::write(
        web.join("package.json"),
        r#"{"dependencies":{"vite":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(web.join("src/web.ts"), "export const web = true;").unwrap();
    std::fs::write(web.join("tests/app.test.ts"), "expect(true).toBe(true);").unwrap();
    std::fs::write(api.join("go.mod"), "module example.com/api\n").unwrap();

    let seeded = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--app-path",
        "apps/web",
        "--persist-planning",
        "--json",
    ]);
    assert_eq!(seeded.status.code(), Some(0));
    let seeded_json: serde_json::Value = serde_json::from_slice(&seeded.stdout).unwrap();
    let run_json = seeded_json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        .to_string();
    let app_key = seeded_json["planning"]["app_key"]
        .as_str()
        .unwrap()
        .to_string();

    let approved = run(&[
        "migrate-approve",
        "--run-json",
        &run_json,
        "--approved-by",
        "tester",
        "--approval-note",
        "approved spec only",
        "--json",
    ]);
    assert_eq!(approved.status.code(), Some(0));
    let approved_json: serde_json::Value = serde_json::from_slice(&approved.stdout).unwrap();
    assert_eq!(approved_json["run_state"], "approved");
    assert_eq!(approved_json["approval_state"], "approved");
    assert_eq!(approved_json["spec_state"], "approved");

    let latest_path = temp
        .path()
        .join(".axhub/spec/apps")
        .join(app_key)
        .join("latest.json");
    let latest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&latest_path).unwrap()).unwrap();
    assert_eq!(latest["approval_state"], "approved");
    assert_eq!(latest["approved_by"], "tester");
}

#[test]
fn cli_migrate_approve_promotes_full_consensus_after_pending_approval() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/db.ts"),
        "export async function rows(db: any) { return db.query('select * from users'); }",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("src/auth.ts"),
        "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
    )
    .unwrap();

    let seeded = run(&[
        "migrate-plan",
        "--dir",
        temp.path().to_str().unwrap(),
        "--persist-planning",
        "--json",
    ]);
    let seeded_json: serde_json::Value = serde_json::from_slice(&seeded.stdout).unwrap();
    let run_json = seeded_json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        .to_string();
    let app_key = seeded_json["planning"]["app_key"]
        .as_str()
        .unwrap()
        .to_string();

    let planner_md = temp.path().join("planner.md");
    let architect_md = temp.path().join("architect.md");
    let critic_md = temp.path().join("critic.md");
    let reviewer_md = temp.path().join("reviewer.md");
    let adr_md = temp.path().join("adr.md");
    std::fs::write(&planner_md, "# Planner").unwrap();
    std::fs::write(&architect_md, "# Architect").unwrap();
    std::fs::write(&critic_md, "# Critic").unwrap();
    std::fs::write(&reviewer_md, "# Reviewer").unwrap();
    std::fs::write(&adr_md, "# ADR").unwrap();

    for (stage, file) in [
        ("planner", &planner_md),
        ("architect", &architect_md),
        ("critic", &critic_md),
        ("adr", &adr_md),
    ] {
        let output = run(&[
            "migrate-stage-write",
            "--run-json",
            &run_json,
            "--stage",
            stage,
            "--markdown-file",
            file.to_str().unwrap(),
            "--json",
        ]);
        assert_eq!(output.status.code(), Some(0));
    }
    let reviewer_output = run(&[
        "migrate-stage-write",
        "--run-json",
        &run_json,
        "--stage",
        "reviewer",
        "--markdown-file",
        reviewer_md.to_str().unwrap(),
        "--run-state",
        "pending_approval",
        "--approval-state",
        "pending_approval",
        "--json",
    ]);
    assert_eq!(reviewer_output.status.code(), Some(0));

    let approved = run(&[
        "migrate-approve",
        "--run-json",
        &run_json,
        "--approved-by",
        "tester",
        "--approval-note",
        "approved consensus",
        "--json",
    ]);
    assert_eq!(approved.status.code(), Some(0));
    let approved_json: serde_json::Value = serde_json::from_slice(&approved.stdout).unwrap();
    assert_eq!(approved_json["run_state"], "approved");
    assert_eq!(approved_json["approval_state"], "approved");
    assert_eq!(approved_json["spec_state"], "approved");

    let latest_path = temp
        .path()
        .join(".axhub/spec/apps")
        .join(app_key)
        .join("latest.json");
    let latest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&latest_path).unwrap()).unwrap();
    assert_eq!(latest["approval_state"], "approved");
    assert_eq!(latest["approved_by"], "tester");

    let approval_path = std::path::Path::new(&run_json)
        .parent()
        .unwrap()
        .join("approval.json");
    let approval: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&approval_path).unwrap()).unwrap();
    assert_eq!(approval["state"], "approved");
    assert!(approval["approved_at"].as_str().is_some());
}

#[test]
fn cli_migrate_plan_emits_six_language_wrapper_previews_without_secret_leak() {
    let temp = tempfile::tempdir().unwrap();

    let node = temp.path().join("apps/node");
    std::fs::create_dir_all(node.join("src")).unwrap();
    std::fs::write(
        node.join("package.json"),
        r#"{"dependencies":{"vite":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        node.join("src/main.ts"),
        "const hardcoded = 'ultra_secret_value_123'; export const boot = () => hardcoded;",
    )
    .unwrap();

    let python = temp.path().join("apps/python");
    std::fs::create_dir_all(python.join("src")).unwrap();
    std::fs::write(
        python.join("pyproject.toml"),
        "[project]\nname='demo'\ndependencies=['fastapi']\n",
    )
    .unwrap();
    std::fs::write(
        python.join("src/app.py"),
        "from fastapi import FastAPI\napp = FastAPI()\n",
    )
    .unwrap();

    let go = temp.path().join("apps/go");
    std::fs::create_dir_all(&go).unwrap();
    std::fs::write(go.join("go.mod"), "module example.com/demo\n").unwrap();
    std::fs::write(go.join("main.go"), "package main\nfunc main() {}\n").unwrap();

    let ruby = temp.path().join("apps/ruby");
    std::fs::create_dir_all(ruby.join("lib")).unwrap();
    std::fs::write(
        ruby.join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'sinatra'\n",
    )
    .unwrap();
    std::fs::write(
        ruby.join("app.rb"),
        "require 'sinatra'\nget('/') { 'ok' }\n",
    )
    .unwrap();

    let java = temp.path().join("apps/java");
    std::fs::create_dir_all(java.join("src/main/java/com/example/demo")).unwrap();
    std::fs::write(java.join("build.gradle"), "plugins { id 'java-library' }\n").unwrap();
    std::fs::write(
        java.join("src/main/java/com/example/demo/Application.java"),
        "package com.example.demo;\npublic class Application {}\n",
    )
    .unwrap();

    let kotlin = temp.path().join("apps/kotlin");
    std::fs::create_dir_all(kotlin.join("src/main/kotlin/com/example/demo")).unwrap();
    std::fs::write(
        kotlin.join("build.gradle.kts"),
        "plugins { kotlin(\"jvm\") version \"2.4.0\" }\n",
    )
    .unwrap();
    std::fs::write(
        kotlin.join("src/main/kotlin/com/example/demo/Application.kt"),
        "package com.example.demo\nclass Application\n",
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
    let sdk_candidates = json["sdk_conversion"]["candidates"].as_array().unwrap();

    let by_path = |path: &str| {
        sdk_candidates
            .iter()
            .find(|candidate| candidate["path"] == path)
            .unwrap()
    };

    assert!(by_path("apps/node")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("@ax-hub/sdk"));
    assert!(!by_path("apps/node")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("ultra_secret_value_123"));
    assert!(by_path("apps/python")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("from axhub_sdk import AxHubClient, TokenType"));
    assert!(by_path("apps/go")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("package main"));
    assert!(by_path("apps/ruby")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("require 'axhub_sdk'"));
    assert!(by_path("apps/java")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("package ai.axhub.sdk;"));
    assert!(by_path("apps/kotlin")["wrapper_preview"]
        .as_str()
        .unwrap()
        .contains("package ai.axhub.sdk"));
}

fn seed_full_consensus_run(temp: &Path) -> (String, String) {
    std::fs::create_dir_all(temp.join("src")).unwrap();
    std::fs::write(
        temp.join("package.json"),
        r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.join("src/db.ts"),
        "export async function rows(db: any) { return db.query('select * from users'); }",
    )
    .unwrap();
    std::fs::write(
        temp.join("src/auth.ts"),
        "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
    )
    .unwrap();
    let seeded = run(&[
        "migrate-plan",
        "--dir",
        temp.to_str().unwrap(),
        "--persist-planning",
        "--json",
    ]);
    assert_eq!(seeded.status.code(), Some(0));
    let seeded_json: serde_json::Value = serde_json::from_slice(&seeded.stdout).unwrap();
    let run_json = seeded_json["planning_persistence"]["paths"]["run_json"]
        .as_str()
        .unwrap()
        .to_string();
    let app_key = seeded_json["planning"]["app_key"]
        .as_str()
        .unwrap()
        .to_string();
    (run_json, app_key)
}

#[test]
fn cli_migrate_wave_plan_rejects_illegal_state_jump() {
    // §7 WaveState guard: a fresh wave must be born `planned` (none -> complete is illegal).
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    let output = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-x",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--state",
        "complete",
        "--json",
    ]);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("planned"),
        "expected wave-birth guard, got stderr: {stderr}"
    );
}

#[test]
fn cli_migrate_wave_plan_complete_requires_artifact_backing() {
    // §8.9: a wave reaching `complete` via the legal 3-call path must have on-disk artifact backing.
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    let planned = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--json",
    ]);
    assert_eq!(planned.status.code(), Some(0));
    let running = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--state",
        "running",
        "--json",
    ]);
    assert_eq!(running.status.code(), Some(0));
    // complete declaring an artifact whose file is absent on disk -> §8.9 reject.
    let completed = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--state",
        "complete",
        "--artifact",
        "missing-backing.md",
        "--json",
    ]);
    assert_ne!(completed.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&completed.stderr);
    assert!(
        stderr.contains("§8.9") || stderr.contains("artifact"),
        "expected §8.9 backing reject, got stderr: {stderr}"
    );
}

#[test]
fn cli_migrate_wave_plan_complete_accepts_present_artifact() {
    // §8.9 positive: a `complete` wave whose declared artifacts exist on disk is accepted.
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    let run_dir = std::path::Path::new(&run_json)
        .parent()
        .unwrap()
        .to_path_buf();
    std::fs::write(run_dir.join("backing-a.md"), "# backing").unwrap();
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--state",
            "running",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    let completed = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--state",
        "complete",
        "--artifact",
        "backing-a.md",
        "--json",
    ]);
    assert_eq!(
        completed.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&completed.stderr)
    );
    let wave_a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("waves/wave-a.json")).unwrap())
            .unwrap();
    assert_eq!(wave_a["state"], "complete");
}

#[test]
fn cli_migrate_wave_plan_idempotent_replan_at_planned_stays_ok() {
    // Idempotency escape: re-running the SAME wave_id at the SAME planned state is a legal upsert.
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    let first = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--json",
    ]);
    assert_eq!(first.status.code(), Some(0));
    let second = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-a",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence",
        "--json",
    ]);
    assert_eq!(
        second.status.code(),
        Some(0),
        "idempotent re-plan must stay exit-0, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
}

#[test]
fn cli_migrate_wave_plan_persisted_complete_not_rerejected() {
    // Candidate-scoping (non-regression): a persisted `complete` wave must NOT be re-validated
    // for §8.9 backing when a second, different wave is written in the same run.
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    let run_dir = std::path::Path::new(&run_json)
        .parent()
        .unwrap()
        .to_path_buf();
    std::fs::write(run_dir.join("backing-a.md"), "# backing").unwrap();
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--state",
            "running",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--state",
            "complete",
            "--artifact",
            "backing-a.md",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    // wave-b is a NEW planned wave in the same run; persisted complete wave-a must survive.
    let wave_b = run(&[
        "migrate-wave-plan",
        "--run-json",
        &run_json,
        "--wave-id",
        "wave-b",
        "--stage-scope",
        "reviewer",
        "--participant",
        &app_key,
        "--independence-proof",
        "disjoint evidence b",
        "--write-target",
        "stages/05-reviewer-b.md",
        "--json",
    ]);
    assert_eq!(
        wave_b.status.code(),
        Some(0),
        "second same-run write must not re-reject persisted complete wave, stderr: {}",
        String::from_utf8_lossy(&wave_b.stderr)
    );
    let wave_a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("waves/wave-a.json")).unwrap())
            .unwrap();
    assert_eq!(wave_a["state"], "complete");
}

#[test]
fn cli_migrate_stage_write_rejects_seal_with_incomplete_wave() {
    // §8.10: sealing running -> pending_approval must reject while a wave is in-progress-unfinished.
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--state",
            "running",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--state",
            "needs_revision",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    let reviewer_md = temp.path().join("reviewer.md");
    std::fs::write(&reviewer_md, "# Reviewer\n\n- approve").unwrap();
    let sealed = run(&[
        "migrate-stage-write",
        "--run-json",
        &run_json,
        "--stage",
        "reviewer",
        "--markdown-file",
        reviewer_md.to_str().unwrap(),
        "--run-state",
        "pending_approval",
        "--approval-state",
        "pending_approval",
        "--json",
    ]);
    assert_ne!(sealed.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&sealed.stderr);
    assert!(
        stderr.contains("§8.10") || stderr.contains("진행 중 wave"),
        "expected §8.10 incomplete-wave reject, got stderr: {stderr}"
    );
}

#[test]
fn cli_migrate_stage_write_seals_with_planned_wave_present() {
    // §8.10 positive: a `planned` wave is acceptable at seal time (only Running/NeedsRevision/Blocked bail).
    let temp = tempfile::tempdir().unwrap();
    let (run_json, app_key) = seed_full_consensus_run(temp.path());
    assert_eq!(
        run(&[
            "migrate-wave-plan",
            "--run-json",
            &run_json,
            "--wave-id",
            "wave-a",
            "--stage-scope",
            "reviewer",
            "--participant",
            &app_key,
            "--independence-proof",
            "disjoint evidence",
            "--json",
        ])
        .status
        .code(),
        Some(0)
    );
    let planner_md = temp.path().join("planner.md");
    let architect_md = temp.path().join("architect.md");
    let critic_md = temp.path().join("critic.md");
    let reviewer_md = temp.path().join("reviewer.md");
    let adr_md = temp.path().join("adr.md");
    std::fs::write(&planner_md, "# Planner\n\n- plan").unwrap();
    std::fs::write(&architect_md, "# Architect\n\n- clear").unwrap();
    std::fs::write(&critic_md, "# Critic\n\n- okay").unwrap();
    std::fs::write(&reviewer_md, "# Reviewer\n\n- approve").unwrap();
    std::fs::write(&adr_md, "# ADR\n\n- chosen").unwrap();
    for (stage, file) in [
        ("planner", &planner_md),
        ("architect", &architect_md),
        ("critic", &critic_md),
        ("adr", &adr_md),
    ] {
        assert_eq!(
            run(&[
                "migrate-stage-write",
                "--run-json",
                &run_json,
                "--stage",
                stage,
                "--markdown-file",
                file.to_str().unwrap(),
                "--json",
            ])
            .status
            .code(),
            Some(0)
        );
    }
    let sealed = run(&[
        "migrate-stage-write",
        "--run-json",
        &run_json,
        "--stage",
        "reviewer",
        "--markdown-file",
        reviewer_md.to_str().unwrap(),
        "--run-state",
        "pending_approval",
        "--approval-state",
        "pending_approval",
        "--json",
    ]);
    assert_eq!(
        sealed.status.code(),
        Some(0),
        "planned-only run must still seal, stderr: {}",
        String::from_utf8_lossy(&sealed.stderr)
    );
    let sealed_json: serde_json::Value = serde_json::from_slice(&sealed.stdout).unwrap();
    assert_eq!(sealed_json["approval_state"], "pending_approval");
}
// ── tenant-resolve (#189) ────────────────────────────────────────────────────

#[cfg(unix)]
fn fake_tenants_axhub(temp: &tempfile::TempDir, tenants_json: &str) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-tenants");
    // Mirror the REAL CLI shapes: `tenants list --json` wraps the rows in a
    // status envelope (`{status, data:{data:[...]}}`); `auth status --json`
    // exposes `tenants[]` with `is_active` (no `current_team_id` field).
    std::fs::write(
        &axhub,
        format!(
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "axhub 0.17.3"; exit 0; fi
if [ "$1 $2 $3" = "tenants list --json" ]; then
  cat <<'AXHUB_TENANTS'
{{"schema_version":"1","status":"ok","data":{{"data":{tenants_json}}}}}
AXHUB_TENANTS
  exit 0
fi
if [ "$1 $2 $3" = "auth status --json" ]; then
  echo '{{"tenants":[{{"is_active":true,"tenant_id":"team-from-preflight","tenant_slug":"preflight"}}]}}'
  exit 0
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
fn fake_slow_tenants_axhub(temp: &tempfile::TempDir) -> std::path::PathBuf {
    let axhub = temp.path().join("axhub-slow-tenants");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1 $2 $3" = "tenants list --json" ]; then sleep 30; exit 0; fi
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
#[test]
fn cli_tenant_resolve_count_one_auto_picks() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_tenants_axhub(
        &temp,
        r#"[{"is_active":true,"role":"tenant_admin","tenant_id":"t-alpha","tenant_slug":"alpha"}]"#,
    );
    let cwd = tempfile::tempdir().unwrap();
    let output = run_in_dir_env(
        &["tenant-resolve", "--json"],
        cwd.path(),
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["tenant"], "t-alpha");
    assert_eq!(json["source"], "auto");
    assert_eq!(json["needs_pick"], false);
}

#[cfg(unix)]
#[test]
fn cli_tenant_resolve_count_many_needs_pick() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_tenants_axhub(
        &temp,
        r#"[{"is_active":true,"tenant_id":"t-a","tenant_slug":"a"},{"is_active":true,"tenant_id":"t-b","tenant_slug":"b"}]"#,
    );
    let cwd = tempfile::tempdir().unwrap();
    let output = run_in_dir_env(
        &["tenant-resolve", "--json"],
        cwd.path(),
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["needs_pick"], true);
    assert_eq!(json["tenant"], "");
    let candidates = json["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    // Candidates are normalized to the {id, slug, name} picker contract so the
    // L2 picker skills never need to know the CLI's tenant_id/tenant_slug keys.
    assert_eq!(candidates[0]["id"], "t-a");
    assert_eq!(candidates[0]["slug"], "a");
    assert_eq!(candidates[0]["name"], "a");
}

#[cfg(unix)]
#[test]
fn cli_tenant_resolve_missing_axhub_fails_open() {
    let cwd = tempfile::tempdir().unwrap();
    let output = run_in_dir_env(
        &["tenant-resolve", "--json"],
        cwd.path(),
        &[("AXHUB_BIN", "/nonexistent/axhub-binary-xyz")],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["tenant"], "");
    assert_eq!(json["needs_pick"], false);
}

#[cfg(unix)]
#[test]
fn cli_tenant_resolve_slow_axhub_times_out_to_empty() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_slow_tenants_axhub(&temp);
    let cwd = tempfile::tempdir().unwrap();
    let output = run_in_dir_env(
        &["tenant-resolve", "--json"],
        cwd.path(),
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["tenant"], "");
}

#[cfg(unix)]
#[test]
fn cli_tenant_resolve_count_zero_uses_preflight_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_tenants_axhub(&temp, "[]");
    let cwd = tempfile::tempdir().unwrap();
    let output = run_in_dir_env(
        &["tenant-resolve", "--json"],
        cwd.path(),
        &[("AXHUB_BIN", axhub.to_str().unwrap())],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["tenant"], "team-from-preflight");
    assert_eq!(json["source"], "preflight");
}

#[cfg(unix)]
#[test]
fn cli_tenant_resolve_cache_hit_short_circuits_without_axhub() {
    let cwd = tempfile::tempdir().unwrap();
    let state_dir = cwd.path().join(".axhub/state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(
        state_dir.join("tenant.json"),
        format!(r#"{{"tenant":"cached-x","source":"picker","ts":{now}}}"#),
    )
    .unwrap();
    // A fresh cache hit must short-circuit BEFORE any axhub call, so a
    // nonexistent AXHUB_BIN must not matter.
    let output = run_in_dir_env(
        &["tenant-resolve", "--json"],
        cwd.path(),
        &[("AXHUB_BIN", "/nonexistent/axhub-binary-xyz")],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    assert_eq!(json["tenant"], "cached-x");
    assert_eq!(json["source"], "picker");
    assert_eq!(json["needs_pick"], false);
}

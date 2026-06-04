use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

use axhub_helpers::axhub_cli::CliOutput;
use axhub_helpers::bootstrap::{
    interpret_apps_create_result, run_bootstrap, AppsCreateDecision, BootstrapState,
    BOOTSTRAP_RECORD_SCHEMA_VERSION,
};
use axhub_helpers::catalog::classify;
use axhub_helpers::consent::{
    format_preauth_deny_hint, mint_token, parse_axhub_command, parse_axhub_commands,
    validate_binding_schema, verify_or_claim_token, verify_token, ConsentBinding,
};
use axhub_helpers::keychain::{
    parse_keyring_value, read_keychain_token_with_runner, CommandOutput,
};
use axhub_helpers::keychain_windows::{
    decode_windows_blob, is_edr_signal, read_windows_keychain_with_runner, WindowsSpawnResult,
    PS_TIMEOUT_MS,
};
use axhub_helpers::list_deployments::{
    list_deployments_cli_args, parse_list_deployments_cli_output, run_list_deployments_with_runner,
    ListDeploymentsArgs, EXIT_LIST_AUTH, EXIT_LIST_NOT_FOUND, EXIT_LIST_OK, EXIT_LIST_TRANSPORT,
};
use axhub_helpers::preflight::{
    extract_semver, parse_auth_status, run_preflight_with_runner, AuthStatus, SpawnResult, EXIT_OK,
    EXIT_USAGE,
};
use axhub_helpers::redact::redact;
use axhub_helpers::resolve::{
    extract_slug_candidate, filter_apps_by_slug, parse_apps_list, parse_resolve_args,
    run_resolve_with_runner, EXIT_NOT_FOUND,
};
use axhub_helpers::spawn::spawn_sync_with_timeout;
use axhub_helpers::telemetry::{
    emit_meta_envelope, reset_cli_version_cache, resolve_cli_version, state_dir,
};
use base64::Engine;
use serde_json::{json, Map, Value};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    saved: HashMap<&'static str, Option<String>>,
    _dir: TempDir,
}
impl EnvGuard {
    fn new(keys: &[&'static str]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let saved = keys.iter().map(|k| (*k, std::env::var(k).ok())).collect();
        Self { saved, _dir: dir }
    }
    fn path(&self, name: &str) -> String {
        self._dir.path().join(name).display().to_string()
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in self.saved.drain() {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

struct CwdGuard {
    saved: std::path::PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
}
impl CwdGuard {
    fn enter(path: &std::path::Path) -> Self {
        let lock = cwd_lock().lock().unwrap();
        let saved = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self { saved, _lock: lock }
    }
}
impl Drop for CwdGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.saved).unwrap();
    }
}

fn base_binding() -> ConsentBinding {
    ConsentBinding {
        tool_call_id: "sess-abc:tc-1".into(),
        action: "deploy_create".into(),
        app_id: "paydrop".into(),
        profile: "prod".into(),
        branch: "main".into(),
        commit_sha: "a3f9c1b".into(),
        context: HashMap::new(),
        synthesized_by_helper: false,
    }
}

fn decode_token_payload(file_path: &str) -> Value {
    let raw = fs::read_to_string(file_path).unwrap();
    let token_file: Value = serde_json::from_str(&raw).unwrap();
    let jwt = token_file.get("jwt").and_then(Value::as_str).unwrap();
    let payload = jwt.split('.').nth(1).unwrap();
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .unwrap();
    serde_json::from_slice(&decoded).unwrap()
}

#[test]
fn redact_matches_typescript_secret_and_unicode_contract() {
    assert_eq!(redact("ａxhub"), "axhub");
    assert_eq!(redact("pay\u{200d}drop"), "paydrop");
    assert_eq!(redact("\u{202a}hello\u{202c}"), "hello");
    assert_eq!(
        redact("Authorization: Bearer abcdef1234567890abcdef"),
        "Authorization: Bearer ***"
    );
    assert_eq!(
        redact("Authorization: Bearer short123"),
        "Authorization: Bearer short123"
    );
    assert_eq!(
        redact("AXHUB_TOKEN=abcdef1234567890abcdef extra"),
        "AXHUB_TOKEN=*** extra"
    );
    assert_eq!(
        redact("test axhub_pat_a1b2c3d4e5f6g7h8i9j0 leak"),
        "test axhub_pat_[redacted] leak"
    );
    assert_eq!(
        redact("test github_pat_11AA22BB33CC44DD55EE66_77FF88GG99HH00II11JJ22KK33LL44MM55NN leak"),
        "test <REDACTED_GH_TOKEN> leak"
    );
    assert_eq!(
        redact(
            "\x1b[32mOK\x1b[0m Bearer abcdef1234567890abcdef AXHUB_TOKEN=xyz1234567890abcdef1234"
        ),
        "OK Bearer *** AXHUB_TOKEN=***"
    );
    assert_eq!(
        redact(r#"{"service_base_url":"https://tenant-a.internal.example/v1","name":"billing"}"#),
        r#"{"service_base_url":"[redacted]","name":"billing"}"#
    );
    assert_eq!(
        redact("service_base_url: https://tenant-a.internal.example/v1"),
        "service_base_url: [redacted]"
    );
}

#[test]
fn spawn_sync_with_timeout_terminates_slow_children() {
    let _cwd_lock = cwd_lock().lock().unwrap();
    let cmd: Vec<&str> = if cfg!(windows) {
        vec![
            "powershell.exe",
            "-NoProfile",
            "-Command",
            "Write-Error warn; Start-Sleep -Milliseconds 800",
        ]
    } else {
        vec!["sh", "-c", "printf warn >&2; sleep 1"]
    };
    let result = spawn_sync_with_timeout(&cmd, 50).unwrap();
    assert_eq!(result.exit_code, None);
    assert!(
        result.stderr.contains("timed out after 50ms"),
        "stderr={}",
        result.stderr
    );
}

#[test]
fn spawn_sync_with_timeout_returns_successful_child_output() {
    let _cwd_lock = cwd_lock().lock().unwrap();
    let cmd: Vec<&str> = if cfg!(windows) {
        vec!["cmd", "/C", "echo timeout-ok"]
    } else {
        vec!["sh", "-c", "printf timeout-ok"]
    };
    let result = spawn_sync_with_timeout(&cmd, 1_000).unwrap();
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.stdout.trim(), "timeout-ok");
    assert_eq!(result.stderr, "");

    let zero_timeout_result = spawn_sync_with_timeout(&cmd, 0).unwrap();
    assert_eq!(zero_timeout_result.exit_code, Some(0));
    assert_eq!(zero_timeout_result.stdout.trim(), "timeout-ok");
}

#[test]
fn catalog_classifies_base_subclassified_and_default_entries() {
    assert!(classify(0, "").emotion.contains("축하해요"));
    assert!(classify(
        64,
        r#"{"error":{"code":"validation.deployment_in_progress"}}"#
    )
    .emotion
    .contains("다른 배포가 먼저 진행 중이에요"));
    assert!(
        classify(66, r#"{"error":{"code":"update.cosign_enforce_failed"}}"#)
            .action
            .contains("IT 보안 담당자")
    );
    // Real CLI cosign-enforce envelope: coarse `code` + fine `subcode`.
    // classify must prefer `subcode` to reach the subclassified entry.
    assert!(classify(
        66,
        r#"{"error":{"code":"other","subcode":"update.cosign_enforce_failed"}}"#
    )
    .action
    .contains("IT 보안 담당자"));
    // Real CLI downgrade-blocked envelope: distinct from scope.downgrade_blocked
    // (binary version downgrade, not deploy-env). Must reach the update.* entry.
    assert!(classify(
        66,
        r#"{"error":{"code":"other","subcode":"update.downgrade_blocked"}}"#
    )
    .emotion
    .contains("더 낮은 버전으로 되돌리려는"));
    assert!(classify(99, "not-json{{").cause.contains("알 수 없는 에러"));
    assert!(
        classify(64, r#"{"error":{"code":"env.prod_force_required"}}"#)
            .action
            .contains("값은 절대")
    );
    assert!(
        classify(5, r#"{"error":{"code":"github.install_not_found"}}"#)
            .button
            .is_some_and(|button| button.contains("GitHub 연결 링크"))
    );
    assert!(classify(
        66,
        r#"{"error":{"code":"profile.endpoint_not_in_allowlist"}}"#
    )
    .emotion
    .contains("허용 목록"));
    assert!(
        classify(64, r#"{"error":{"code":"apis.call_consent_required"}}"#)
            .cause
            .contains("서버 상태")
    );
    // spec 004: helper-output exits (65/67) normalize into the CLI space at the
    // integration level too — not only in the catalog.rs unit test. classify()
    // must route helper auth=65 / not_found=67 to the same template as 4/5.
    assert!(classify(65, r#"{"error":{"code":"auth"}}"#)
        .action
        .contains("다시 로그인"));
    assert!(
        classify(67, r#"{"error":{"code":"github.install_not_found"}}"#)
            .button
            .is_some_and(|button| button.contains("GitHub 연결 링크"))
    );
}

#[test]
fn preflight_semver_auth_and_exit_precedence_match_ts() {
    assert_eq!(
        extract_semver("axhub 1.2.3-rc.1+build (commit abc)"),
        Some("1.2.3".into())
    );
    assert!(
        matches!(parse_auth_status(r#"{"code":"auth.expired","detail":"expired"}"#), AuthStatus::Error { code, .. } if code == "auth.expired")
    );
    let run = run_preflight_with_runner(|cmd| {
        match cmd {
        ["axhub", "--version"] => SpawnResult { exit_code: EXIT_OK, stdout: "axhub 0.17.3".into(), stderr: String::new() },
        ["axhub", "auth", "status", "--json"] => SpawnResult { exit_code: EXIT_OK, stdout: r#"{"user_email":"u@example.com","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy"],"current_team_id":"acme","tenants":[{"tenant_slug":"other"}]}"#.into(), stderr: String::new() },
        _ => SpawnResult { exit_code: 1, stdout: String::new(), stderr: String::new() },
    }
    });
    assert_eq!(run.exit_code, EXIT_OK);
    assert_eq!(run.output.user_email.as_deref(), Some("u@example.com"));
    assert_eq!(run.output.current_team_id.as_deref(), Some("acme"));
    let too_new = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 1.0.0".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"code":"auth.expired"}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });
    assert_eq!(too_new.exit_code, EXIT_USAGE);
    assert!(too_new.output.cli_too_new);
}

#[test]
fn preflight_admits_0_x_line_and_rejects_1_0_exclusive_max() {
    for version in ["0.17.3", "0.18.0", "0.99.0"] {
        let run = run_preflight_with_runner(|cmd| {
            match cmd {
            ["axhub", "--version"] => SpawnResult {
                exit_code: EXIT_OK,
                stdout: format!("axhub {version}"),
                stderr: String::new(),
            },
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: EXIT_OK,
                stdout: r#"{"user_email":"u@example.com","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["read"]}"#.into(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        }
        });
        assert_eq!(run.exit_code, EXIT_OK, "version {version}");
        assert!(run.output.in_range, "version {version}");
        assert!(!run.output.cli_too_new, "version {version}");
    }

    let too_old = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.17.2".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"user_email":"u@example.com","scopes":["read"]}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });
    assert_eq!(too_old.exit_code, EXIT_USAGE);
    assert!(too_old.output.cli_too_old);

    let too_new = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 1.0.0".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"user_email":"u@example.com","scopes":["read"]}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });
    assert_eq!(too_new.exit_code, EXIT_USAGE);
    assert!(too_new.output.cli_too_new);
}

#[test]
fn preflight_covers_auth_shapes_env_cache_and_cli_absence() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&[
        "HOME",
        "AXHUB_PROFILE",
        "AXHUB_ENDPOINT",
        "AXHUB_APP_SLUG",
        "AXHUB_PLUGIN_VERSION",
        "AXHUB_BIN",
    ]);
    assert!(
        matches!(parse_auth_status("not json"), AuthStatus::Error { code, .. } if code == "parse_error")
    );
    assert!(
        matches!(parse_auth_status(r#"{"unexpected":true}"#), AuthStatus::Error { code, .. } if code == "unknown_shape")
    );
    assert!(
        matches!(parse_auth_status(r#"{"user_email":"u@example.com","scopes":["deploy",1,null]}"#), AuthStatus::Ok { user_id, scopes, .. } if user_id == 0 && scopes == vec!["deploy"])
    );
    assert!(
        matches!(parse_auth_status(r#"{"user_email":"u@example.com","scopes":["deploy"],"current_tenant":{"tenant_slug":"acme"},"tenants":[{"tenant_slug":"other"}]}"#), AuthStatus::Ok { current_team_id, .. } if current_team_id.as_deref() == Some("acme"))
    );
    assert!(
        matches!(parse_auth_status(r#"{"user_email":"u@example.com","scopes":["deploy"],"tenants":[{"tenant_slug":"acme"}]}"#), AuthStatus::Ok { current_team_id, .. } if current_team_id.is_none())
    );

    std::env::set_var("HOME", guard.path("home"));
    std::env::set_var("AXHUB_PROFILE", "prod");
    std::env::set_var("AXHUB_ENDPOINT", "https://example.test");
    std::env::set_var("AXHUB_PLUGIN_VERSION", "9.9.9");
    std::env::remove_var("AXHUB_APP_SLUG");
    fs::create_dir_all(guard.path("home/.cache/axhub-plugin")).unwrap();
    fs::write(
        guard.path("home/.cache/axhub-plugin/last-deploy.json"),
        r#"{"deployment_id":"dep-1","status":"active","app_slug":"cached-app"}"#,
    )
    .unwrap();

    let auth_expired = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.17.3".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"code":"auth.expired","detail":"expired"}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });
    assert_eq!(auth_expired.exit_code, axhub_helpers::preflight::EXIT_AUTH);
    assert_eq!(
        auth_expired.output.auth_error_code.as_deref(),
        Some("auth.expired")
    );
    assert_eq!(auth_expired.output.profile.as_deref(), Some("prod"));
    assert_eq!(
        auth_expired.output.endpoint.as_deref(),
        Some("https://example.test")
    );
    assert_eq!(
        auth_expired.output.current_app.as_deref(),
        Some("cached-app")
    );
    assert_eq!(auth_expired.output.last_deploy_id.as_deref(), Some("dep-1"));
    assert_eq!(auth_expired.output.plugin_version, "9.9.9");

    std::env::remove_var("AXHUB_PLUGIN_VERSION");
    let default_plugin_version = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.17.3".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"user_email":"u@example.com","scopes":[]}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });
    assert_eq!(
        default_plugin_version.output.plugin_version,
        env!("CARGO_PKG_VERSION")
    );

    std::env::set_var("AXHUB_APP_SLUG", "env-app");
    let absent = run_preflight_with_runner(|_cmd| SpawnResult {
        exit_code: 127,
        stdout: String::new(),
        stderr: "not found".into(),
    });
    assert_eq!(absent.exit_code, EXIT_USAGE);
    assert!(!absent.output.cli_present);
    // v0.9.5: exit 127 + "not found" stderr is classified as cli_not_found (more
    // specific than the legacy blanket cli_unavailable). SKILL wrappers route
    // this to `/axhub:install-cli` instead of mixing it with config_corrupted /
    // runtime_error fixes.
    assert_eq!(
        absent.output.auth_error_code.as_deref(),
        Some("cli_not_found")
    );
    assert_eq!(absent.output.current_app.as_deref(), Some("env-app"));

    let too_old = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.0.9".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"user_email":"u@example.com","scopes":[]}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });
    assert_eq!(too_old.exit_code, EXIT_USAGE);
    assert!(too_old.output.cli_too_old);
}

#[test]
fn preflight_current_app_prefers_env_manifest_then_cache() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["HOME", "AXHUB_APP_SLUG", "AXHUB_BIN"]);
    let _cwd = CwdGuard::enter(guard._dir.path());
    std::env::set_var("HOME", guard.path("home"));
    std::env::remove_var("AXHUB_APP_SLUG");
    fs::create_dir_all(guard.path("home/.cache/axhub-plugin")).unwrap();
    fs::write(
        guard.path("home/.cache/axhub-plugin/last-deploy.json"),
        r#"{"deployment_id":"dep-1","status":"active","app_slug":"cached-app"}"#,
    )
    .unwrap();
    // v0.9.5: cache fallback now requires cwd to look like a project directory
    // (any of .git / package.json / Cargo.toml / apphub.yaml / etc.). Without a
    // marker the preflight emits current_app=None to avoid stale "현재 앱:
    // <slug>" rendering in unrelated empty directories. Test cwd is a fresh
    // tempdir — touch `.git/` so the cache fallback assertions below preserve
    // their original intent (manifest > env > cache priority resolution).
    fs::create_dir(".git").unwrap();

    let ok_runner = |cmd: &[&str]| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.17.3".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"user_email":"u@example.com","scopes":["read"]}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    };

    fs::write("apphub.yaml", "name: yaml-app\n").unwrap();
    let manifest = run_preflight_with_runner(ok_runner);
    assert_eq!(manifest.output.current_app.as_deref(), Some("yaml-app"));

    std::env::set_var("AXHUB_APP_SLUG", "env-app");
    let env_override = run_preflight_with_runner(ok_runner);
    assert_eq!(env_override.output.current_app.as_deref(), Some("env-app"));

    std::env::remove_var("AXHUB_APP_SLUG");
    fs::remove_file("apphub.yaml").unwrap();
    fs::write("axhub.yaml", "slug: legacy-app\n").unwrap();
    let legacy_manifest = run_preflight_with_runner(ok_runner);
    assert_eq!(
        legacy_manifest.output.current_app.as_deref(),
        Some("legacy-app")
    );

    fs::remove_file("axhub.yaml").unwrap();
    fs::write("apphub.yaml", "name: not a slug\n").unwrap();
    let invalid_manifest = run_preflight_with_runner(ok_runner);
    assert_eq!(
        invalid_manifest.output.current_app.as_deref(),
        Some("cached-app")
    );

    fs::remove_file("apphub.yaml").unwrap();
    let cache_fallback = run_preflight_with_runner(ok_runner);
    assert_eq!(
        cache_fallback.output.current_app.as_deref(),
        Some("cached-app")
    );
}

#[test]
fn preflight_uses_xdg_cache_home_for_last_deploy_cache() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["HOME", "XDG_CACHE_HOME", "AXHUB_APP_SLUG", "AXHUB_BIN"]);
    let _cwd = CwdGuard::enter(guard._dir.path());
    std::env::set_var("HOME", guard.path("home"));
    std::env::set_var("XDG_CACHE_HOME", guard.path("xdg-cache"));
    std::env::remove_var("AXHUB_APP_SLUG");
    fs::create_dir_all(guard.path("xdg-cache/axhub-plugin")).unwrap();
    fs::write(
        guard.path("xdg-cache/axhub-plugin/last-deploy.json"),
        r#"{"deployment_id":"dep-xdg","status":"active","app_slug":"xdg-app"}"#,
    )
    .unwrap();
    // v0.9.5: cache fallback gated on cwd project marker. See sibling test
    // `preflight_current_app_prefers_env_manifest_then_cache` for context.
    fs::create_dir(".git").unwrap();

    let run = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.17.3".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: r#"{"user_email":"u@example.com","scopes":["read"]}"#.into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        },
    });

    assert_eq!(run.output.current_app.as_deref(), Some("xdg-app"));
    assert_eq!(run.output.last_deploy_id.as_deref(), Some("dep-xdg"));
}

#[test]
fn resolve_filters_apps_and_preserves_git_context_for_errors() {
    assert_eq!(
        extract_slug_candidate("paydrop 배포해"),
        Some("paydrop".into())
    );
    let apps = parse_apps_list(
        r#"[{"id":1,"slug":"paydrop"},{"id":2,"slug":"paydrop-admin"},{"id":3,"slug":"other"}]"#,
    )
    .unwrap();
    assert_eq!(filter_apps_by_slug(&apps, "paydrop").len(), 2);
    let envelope_apps =
        parse_apps_list(r#"{"apps":[{"id":7,"slug":"paydrop","name":"Paydrop"}],"total":1}"#)
            .unwrap();
    assert_eq!(envelope_apps[0].slug, "paydrop");
    assert_eq!(envelope_apps[0].name.as_deref(), Some("Paydrop"));
    let data_apps =
        parse_apps_list(r#"{"data":[{"id":8,"slug":"paydrop-live","name":"Paydrop Live"}]}"#)
            .unwrap();
    assert_eq!(data_apps[0].id, "8");
    assert_eq!(data_apps[0].slug, "paydrop-live");
    let run = run_resolve_with_runner(
        &["--user-utterance".into(), "paydrop 배포해".into()],
        |cmd| match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"u@example.com","scopes":["deploy"]}"#.into(),
                stderr: String::new(),
            },
            ["axhub", "apps", "list", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"data":[{"id":42,"slug":"paydrop"}],"total":1}"#.into(),
                stderr: String::new(),
            },
            ["git", "rev-parse", "--is-inside-work-tree"] => SpawnResult {
                exit_code: 0,
                stdout: "true\n".into(),
                stderr: String::new(),
            },
            ["git", "branch", "--show-current"] => SpawnResult {
                exit_code: 0,
                stdout: "main\n".into(),
                stderr: String::new(),
            },
            ["git", "rev-parse", "HEAD"] => SpawnResult {
                exit_code: 0,
                stdout: "abc123\n".into(),
                stderr: String::new(),
            },
            ["git", "log", "-1", "--pretty=%s"] => SpawnResult {
                exit_code: 0,
                stdout: "msg\n".into(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        },
    );
    assert_eq!(run.exit_code, EXIT_OK);
    assert_eq!(run.output.app_id, Some("42".to_string()));
    assert_eq!(run.output.branch.as_deref(), Some("main"));
}

#[test]
fn resolve_marks_non_git_repositories_as_init_needed() {
    let run = run_resolve_with_runner(
        &["--user-utterance".into(), "paydrop 배포해".into()],
        |cmd| match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"u@example.com","scopes":["deploy"]}"#.into(),
                stderr: String::new(),
            },
            ["axhub", "apps", "list", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"[{"id":42,"slug":"paydrop"}]"#.into(),
                stderr: String::new(),
            },
            ["git", "rev-parse", "--is-inside-work-tree"] => SpawnResult {
                exit_code: 128,
                stdout: String::new(),
                stderr: "fatal: not a git repository".into(),
            },
            ["git", "branch", "--show-current"]
            | ["git", "rev-parse", "HEAD"]
            | ["git", "log", "-1", "--pretty=%s"] => SpawnResult {
                exit_code: 128,
                stdout: String::new(),
                stderr: "fatal: not a git repository".into(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        },
    );

    assert_eq!(run.exit_code, EXIT_OK);
    assert_eq!(run.output.app_id, Some("42".to_string()));
    assert_eq!(run.output.branch, None);
    assert_eq!(run.output.commit_sha, None);
    assert!(!run.output.git_repo);
    assert!(!run.output.git_has_commit);
    assert!(run.output.git_init_needed);
}

#[test]
fn resolve_uses_manifest_name_slug_when_utterance_has_no_candidate() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["AXHUB_PROFILE", "AXHUB_ENDPOINT", "AXHUB_BIN"]);
    let _cwd = CwdGuard::enter(guard._dir.path());
    fs::write(
        guard._dir.path().join("apphub.yaml"),
        "name: nextjs-axhub\nframework: nextjs\n",
    )
    .unwrap();

    let run =
        run_resolve_with_runner(
            &["--user-utterance".into(), "배포해줘".into()],
            |cmd| match cmd {
                ["axhub", "auth", "status", "--json"] => SpawnResult {
                    exit_code: 0,
                    stdout: r#"{"user_email":"u@example.com","scopes":["deploy"]}"#.into(),
                    stderr: String::new(),
                },
                ["axhub", "apps", "list", "--json"] => SpawnResult {
                    exit_code: 0,
                    stdout: r#"[{"id":165,"slug":"nextjs-axhub"}]"#.into(),
                    stderr: String::new(),
                },
                ["git", "rev-parse", "--is-inside-work-tree"] => SpawnResult {
                    exit_code: 0,
                    stdout: "true\n".into(),
                    stderr: String::new(),
                },
                ["git", "branch", "--show-current"] => SpawnResult {
                    exit_code: 0,
                    stdout: "main\n".into(),
                    stderr: String::new(),
                },
                ["git", "rev-parse", "HEAD"] => SpawnResult {
                    exit_code: 0,
                    stdout: "4fd1140\n".into(),
                    stderr: String::new(),
                },
                ["git", "log", "-1", "--pretty=%s"] => SpawnResult {
                    exit_code: 0,
                    stdout: "init\n".into(),
                    stderr: String::new(),
                },
                _ => SpawnResult {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                },
            },
        );

    assert_eq!(run.exit_code, EXIT_OK);
    assert_eq!(run.output.candidate_slug.as_deref(), Some("nextjs-axhub"));
    assert_eq!(run.output.app_id, Some("165".to_string()));
    assert_eq!(run.output.app_slug.as_deref(), Some("nextjs-axhub"));
    assert_eq!(run.output.branch.as_deref(), Some("main"));
    assert_eq!(run.output.commit_sha.as_deref(), Some("4fd1140"));
}

#[test]
fn resolve_uses_git_remote_repo_name_when_no_utterance_or_manifest_slug() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["AXHUB_PROFILE", "AXHUB_ENDPOINT", "AXHUB_BIN"]);
    let _cwd = CwdGuard::enter(guard._dir.path());
    // No apphub.yaml/axhub.yaml here -> manifest yields no slug, so the candidate
    // must come from the github `origin` remote (repo name == app slug). Also
    // exercises the `{"items":[{"id":"<uuid>"}]}` apps-list shape end-to-end.

    let run = run_resolve_with_runner(&["--user-utterance".into(), "배포해줘".into()], |cmd| {
        match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"u@example.com","scopes":["deploy"]}"#.into(),
                stderr: String::new(),
            },
            ["axhub", "apps", "list", "--json"] => SpawnResult {
                exit_code: 0,
                stdout:
                    r#"{"items":[{"id":"f349a303-a294-48c8-96f3-d85d85f5faa7","slug":"employee-directory"}],"total":1}"#
                        .into(),
                stderr: String::new(),
            },
            ["git", "config", "--get", "remote.origin.url"] => SpawnResult {
                exit_code: 0,
                stdout: "https://github.com/jocoding-ax-partners/employee-directory.git\n".into(),
                stderr: String::new(),
            },
            ["git", "rev-parse", "--is-inside-work-tree"] => SpawnResult {
                exit_code: 0,
                stdout: "true\n".into(),
                stderr: String::new(),
            },
            ["git", "branch", "--show-current"] => SpawnResult {
                exit_code: 0,
                stdout: "main\n".into(),
                stderr: String::new(),
            },
            ["git", "rev-parse", "HEAD"] => SpawnResult {
                exit_code: 0,
                stdout: "abc123\n".into(),
                stderr: String::new(),
            },
            ["git", "log", "-1", "--pretty=%s"] => SpawnResult {
                exit_code: 0,
                stdout: "init\n".into(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        }
    });

    assert_eq!(run.exit_code, EXIT_OK);
    assert_eq!(
        run.output.candidate_slug.as_deref(),
        Some("employee-directory")
    );
    assert_eq!(
        run.output.app_id.as_deref(),
        Some("f349a303-a294-48c8-96f3-d85d85f5faa7")
    );
    assert_eq!(run.output.app_slug.as_deref(), Some("employee-directory"));
    assert!(run.output.git_repo);
}

#[test]
fn preflight_current_app_prefers_git_remote_over_stale_global_cache() {
    let _lock = env_lock().lock().unwrap();
    // Clear AXHUB_APP_SLUG so the env source doesn't shadow the remote path, and
    // enter an empty tempdir so no apphub.yaml/axhub.yaml manifest exists. Then
    // current_app must come from the git `origin` repo name, NOT the global
    // last-deploy cache (which would leak a different app's slug into this dir).
    let guard = EnvGuard::new(&[
        "AXHUB_APP_SLUG",
        "AXHUB_PROFILE",
        "AXHUB_ENDPOINT",
        "AXHUB_BIN",
    ]);
    let _cwd = CwdGuard::enter(guard._dir.path());

    let run = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: 0,
            stdout: "axhub 0.17.3\n".into(),
            stderr: String::new(),
        },
        ["axhub", "auth", "status", "--json"] => SpawnResult {
            exit_code: 0,
            stdout: r#"{"user_email":"u@example.com","scopes":["deploy"]}"#.into(),
            stderr: String::new(),
        },
        ["git", "config", "--get", "remote.origin.url"] => SpawnResult {
            exit_code: 0,
            stdout: "https://github.com/jocoding-ax-partners/employee-directory.git\n".into(),
            stderr: String::new(),
        },
        _ => SpawnResult {
            exit_code: 0,
            stdout: "[]".into(),
            stderr: String::new(),
        },
    });

    assert_eq!(
        run.output.current_app.as_deref(),
        Some("employee-directory")
    );
}

#[test]
fn resolve_covers_arg_parsing_auth_parse_ambiguity_and_not_found_paths() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["AXHUB_PROFILE", "AXHUB_ENDPOINT", "AXHUB_BIN"]);
    std::env::set_var("AXHUB_PROFILE", "prod");
    std::env::set_var("AXHUB_ENDPOINT", "https://example.test");

    let parsed = parse_resolve_args(&[
        "--intent".into(),
        "deploy".into(),
        "--ignored".into(),
        "--user-utterance".into(),
        "paydrop 배포".into(),
    ]);
    assert_eq!(parsed.intent.as_deref(), Some("deploy"));
    assert_eq!(parsed.user_utterance, "paydrop 배포");
    assert_eq!(
        extract_slug_candidate("ｐａｙｄｒｏｐ 프로덕션에 올려"),
        Some("paydrop".into())
    );
    assert_eq!(extract_slug_candidate("그거 배포해줘"), None);
    assert!(parse_apps_list("not json").is_none());
    assert!(parse_apps_list(r#"{"apps":"not-array","total":1}"#).is_none());
    // id is normalized to String now (UUID migration): a string id is valid, so
    // the dropped entry must be one with a genuinely absent/null id, not merely
    // non-numeric. Keeps the intent: one valid + one malformed -> len 1.
    assert_eq!(
        parse_apps_list(r#"[{"id":1,"slug":"paydrop"},{"id":null,"slug":"skip"}]"#)
            .unwrap()
            .len(),
        1
    );

    let auth_parse_error = run_resolve_with_runner(
        &["--user-utterance".into(), "paydrop".into()],
        |cmd| match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: "not json".into(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        },
    );
    assert_eq!(
        auth_parse_error.exit_code,
        axhub_helpers::preflight::EXIT_AUTH
    );
    assert_eq!(
        auth_parse_error.output.error.as_deref(),
        Some("auth_parse_error")
    );

    let apps_parse = run_resolve_with_runner(
        &["--user-utterance".into(), "paydrop".into()],
        |cmd| match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"u@example.com"}"#.into(),
                stderr: String::new(),
            },
            ["axhub", "apps", "list", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: "{}".into(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        },
    );
    assert_eq!(apps_parse.exit_code, EXIT_NOT_FOUND);
    assert_eq!(
        apps_parse.output.error.as_deref(),
        Some("apps_list_parse_error")
    );

    let ambiguous = run_resolve_with_runner(
        &["--user-utterance".into(), "paydrop".into()],
        |cmd| match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"u@example.com"}"#.into(),
                stderr: String::new(),
            },
            ["axhub", "apps", "list", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"[{"id":1,"slug":"paydrop"},{"id":2,"slug":"paydrop-admin"}]"#.into(),
                stderr: String::new(),
            },
            ["git", "branch", "--show-current"] => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
            ["git", "rev-parse", "HEAD"] => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
            ["git", "log", "-1", "--pretty=%s"] => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        },
    );
    assert_eq!(ambiguous.exit_code, EXIT_USAGE);
    assert_eq!(ambiguous.output.error.as_deref(), Some("app_ambiguous"));
    assert_eq!(ambiguous.output.matched_apps.len(), 2);
    assert_eq!(ambiguous.output.profile.as_deref(), Some("prod"));
    assert_eq!(
        ambiguous.output.endpoint.as_deref(),
        Some("https://example.test")
    );

    let no_candidate = run_resolve_with_runner(
        &["--user-utterance".into(), "배포해줘".into()],
        |cmd| match cmd {
            ["axhub", "auth", "status", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"u@example.com"}"#.into(),
                stderr: String::new(),
            },
            ["axhub", "apps", "list", "--json"] => SpawnResult {
                exit_code: 0,
                stdout: r#"[{"id":1,"slug":"paydrop"}]"#.into(),
                stderr: String::new(),
            },
            _ => SpawnResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        },
    );
    assert_eq!(no_candidate.exit_code, EXIT_NOT_FOUND);
    assert_eq!(
        no_candidate.output.error.as_deref(),
        Some("no_candidate_slug")
    );

    let contains_match = filter_apps_by_slug(
        &[axhub_helpers::resolve::AppRecord {
            id: "1".into(),
            slug: "admin-paydrop".into(),
            name: None,
            github_repo_url: None,
        }],
        "pay",
    );
    assert_eq!(contains_match[0].slug, "admin-paydrop");
    drop(guard);
}

#[test]
fn list_deployments_wraps_current_cli_deploy_list() {
    let args = ListDeploymentsArgs {
        app_id: "paydrop".into(),
        limit: Some(3),
    };
    assert_eq!(
        list_deployments_cli_args(&args),
        vec![
            "--json",
            "deploy",
            "list",
            "--app",
            "paydrop",
            "--page-size",
            "3"
        ]
    );

    let got = run_list_deployments_with_runner(args, |argv| {
        assert_eq!(
            argv,
            &[
                "--json".to_string(),
                "deploy".to_string(),
                "list".to_string(),
                "--app".to_string(),
                "paydrop".to_string(),
                "--page-size".to_string(),
                "3".to_string(),
            ]
        );
        CliOutput {
            exit_code: 0,
            stdout: r#"{"items":[{"id":"dep_7","app_id":"app_uuid","status":"running","commit_sha":"abc","started_at":"2026-04-29T00:00:00Z"}]}"#.into(),
            stderr: String::new(),
            timed_out: false,
        }
    });
    assert_eq!(got.exit_code, EXIT_LIST_OK);
    assert_eq!(got.endpoint_used, "cli");
    assert_eq!(got.deployments[0].id, "dep_7");
    assert_eq!(got.deployments[0].app_id, "app_uuid");
    assert_eq!(got.deployments[0].status, "running");
}

#[test]
fn list_deployments_covers_cli_envelope_shapes_and_error_matrix() {
    let args = ListDeploymentsArgs {
        app_id: "paydrop".into(),
        limit: None,
    };

    let enveloped = parse_list_deployments_cli_output(
        &args,
        CliOutput {
            exit_code: 0,
            stdout: r#"{"schema_version":"1","status":"ok","data":{"items":[{"id":"dep_1","status":"succeeded","started_at":"2026-04-29T00:00:00Z"}]}}"#.into(),
            stderr: String::new(),
            timed_out: false,
        },
    );
    assert_eq!(enveloped.exit_code, EXIT_LIST_OK);
    assert_eq!(enveloped.deployments[0].app_id, "paydrop");

    for (exit_code, error_json, expected_exit, expected_code) in [
        (
            4,
            r#"{"schema_version":"1","status":"error","error":{"code":"auth","hint":"login"}}"#,
            EXIT_LIST_AUTH,
            "auth",
        ),
        (
            5,
            r#"{"schema_version":"1","status":"error","error":{"code":"not_found"}}"#,
            EXIT_LIST_NOT_FOUND,
            "not_found",
        ),
        (
            1,
            r#"{"schema_version":"1","status":"error","error":{"subcode":"transport.network_error"}}"#,
            EXIT_LIST_TRANSPORT,
            "transport.network_error",
        ),
    ] {
        let got = parse_list_deployments_cli_output(
            &args,
            CliOutput {
                exit_code,
                stdout: error_json.into(),
                stderr: String::new(),
                timed_out: false,
            },
        );
        assert_eq!(got.exit_code, expected_exit);
        assert_eq!(got.error_code.as_deref(), Some(expected_code));
    }

    let invalid_json = parse_list_deployments_cli_output(
        &args,
        CliOutput {
            exit_code: 0,
            stdout: "not json".into(),
            stderr: String::new(),
            timed_out: false,
        },
    );
    assert_eq!(invalid_json.exit_code, EXIT_LIST_TRANSPORT);
    assert_eq!(
        invalid_json.error_code.as_deref(),
        Some("response.invalid_json")
    );

    let timed_out = parse_list_deployments_cli_output(
        &args,
        CliOutput {
            exit_code: 124,
            stdout: String::new(),
            stderr: String::new(),
            timed_out: true,
        },
    );
    assert_eq!(timed_out.exit_code, EXIT_LIST_TRANSPORT);
    assert_eq!(timed_out.error_code.as_deref(), Some("transport.timeout"));

    let all_statuses = parse_list_deployments_cli_output(
        &args,
        CliOutput {
            exit_code: 0,
            stdout: r#"{"items":[
                {"id":"1","app_id":"app","status":0,"commit_sha":"a","created_at":"t"},
                {"id":"2","app_id":"app","status":1,"commit_sha":"b","commit_message":"m","branch":"dev","created_at":"t"},
                {"id":"3","app_id":"app","status":2,"commit_sha":"c","created_at":"t"},
                {"id":"4","app_id":"app","status":3,"commit_sha":"d","created_at":"t"},
                {"id":"5","app_id":"app","status":4,"commit_sha":"e","created_at":"t"},
                {"id":"6","app_id":"app","status":5,"commit_sha":"f","created_at":"t"},
                {"id":"7","app_id":"app","status":99,"commit_sha":"g","created_at":"t"}
            ]}"#.into(),
            stderr: String::new(),
            timed_out: false,
        },
    );
    assert_eq!(
        all_statuses
            .deployments
            .iter()
            .map(|d| d.status.as_str())
            .collect::<Vec<_>>(),
        vec![
            "pending",
            "building",
            "deploying",
            "active",
            "failed",
            "stopped",
            "unknown_99",
        ]
    );
    assert_eq!(all_statuses.deployments[0].commit_message, "");
    assert_eq!(all_statuses.deployments[1].branch, "dev");
}

#[test]
fn consent_locks_zero_leeway_binding_mismatch_and_parser_hardening() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["XDG_STATE_HOME", "XDG_RUNTIME_DIR", "CLAUDE_SESSION_ID"]);
    std::env::set_var("XDG_STATE_HOME", guard.path("state"));
    std::env::set_var("XDG_RUNTIME_DIR", guard.path("runtime"));
    std::env::set_var("CLAUDE_SESSION_ID", "test-session");

    let binding = base_binding();
    let minted = mint_token(binding.clone(), 60).unwrap();
    assert!(minted.file_path.contains("consent-test-session.json"));
    assert!(verify_token(binding.clone()).valid);
    let mut wrong = binding.clone();
    wrong.app_id = "otherapp".into();
    let result = verify_token(wrong);
    assert!(!result.valid);
    assert_eq!(result.reason.as_deref(), Some("binding_mismatch:app_id"));

    mint_token(binding.clone(), -1).unwrap();
    assert_eq!(
        verify_token(binding.clone()).reason.as_deref(),
        Some("token_expired")
    );
    mint_token(binding.clone(), 0).unwrap();
    assert_eq!(
        verify_token(binding).reason.as_deref(),
        Some("token_expired")
    );

    let parsed = parse_axhub_command(
        r#"FOO=bar bash -c "(axhub deploy create --app paydrop --branch main --commit abc1234 --json)""#,
    );
    assert!(parsed.is_destructive);
    assert_eq!(parsed.action.as_deref(), Some("deploy_create"));
    assert_eq!(parsed.app_id.as_deref(), Some("paydrop"));
}

#[test]
fn consent_pending_token_allows_future_bash_tool_call_once_without_session_env() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["XDG_STATE_HOME", "XDG_RUNTIME_DIR", "CLAUDE_SESSION_ID"]);
    std::env::set_var("XDG_STATE_HOME", guard.path("state"));
    std::env::set_var("XDG_RUNTIME_DIR", guard.path("runtime"));
    std::env::remove_var("CLAUDE_SESSION_ID");

    let mut pending_binding = base_binding();
    pending_binding.tool_call_id = "model-does-not-know-next-bash-id".into();
    let minted = mint_token(pending_binding.clone(), 60).unwrap();
    assert!(minted.file_path.contains("consent-pending-"));
    assert!(std::path::Path::new(&minted.file_path).exists());
    assert_eq!(
        verify_token(pending_binding).reason.as_deref(),
        Some("session_id_missing")
    );

    let mut actual = base_binding();
    actual.tool_call_id = "actual-session:toolu_123".into();
    let mut wrong_branch = actual.clone();
    wrong_branch.branch = "dev".into();
    let rejected = verify_or_claim_token(wrong_branch);
    assert!(!rejected.valid);
    assert!(std::path::Path::new(&minted.file_path).exists());

    assert!(verify_or_claim_token(actual.clone()).valid);
    assert!(!std::path::Path::new(&minted.file_path).exists());
    let second_try = verify_or_claim_token(actual);
    assert!(!second_try.valid);
}

#[test]
fn consent_pending_token_allows_future_bash_tool_call_with_session_env_when_explicit_pending() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["XDG_STATE_HOME", "XDG_RUNTIME_DIR", "CLAUDE_SESSION_ID"]);
    std::env::set_var("XDG_STATE_HOME", guard.path("state"));
    std::env::set_var("XDG_RUNTIME_DIR", guard.path("runtime"));
    std::env::set_var("CLAUDE_SESSION_ID", "current-claude-session");

    let mut pending_binding = base_binding();
    pending_binding.tool_call_id = "pending".into();
    let minted = mint_token(pending_binding, 60).unwrap();
    assert!(minted.file_path.contains("consent-pending-"));
    assert!(std::path::Path::new(&minted.file_path).exists());

    let mut actual = base_binding();
    actual.tool_call_id = "actual-session:toolu_456".into();
    assert!(verify_or_claim_token(actual).valid);
    assert!(!std::path::Path::new(&minted.file_path).exists());
}

#[test]
fn consent_binding_accepts_context_and_backfills_legacy_tokens() {
    let with_context: ConsentBinding = serde_json::from_value(json!({
        "tool_call_id": "sess-abc:tc-1",
        "action": "env_set",
        "app_id": "paydrop",
        "profile": "prod",
        "branch": "",
        "commit_sha": "",
        "context": {"key": "DATABASE_URL"}
    }))
    .unwrap();
    assert_eq!(
        with_context.context.get("key").map(String::as_str),
        Some("DATABASE_URL")
    );

    let legacy: ConsentBinding = serde_json::from_value(json!({
        "tool_call_id": "sess-abc:tc-1",
        "action": "deploy_create",
        "app_id": "paydrop",
        "profile": "prod",
        "branch": "main",
        "commit_sha": "a3f9c1b"
    }))
    .unwrap();
    assert!(legacy.context.is_empty());
    assert!(!legacy.synthesized_by_helper);
}

#[test]
fn consent_synthesized_by_helper_claim_is_audit_only() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&["XDG_STATE_HOME", "XDG_RUNTIME_DIR", "CLAUDE_SESSION_ID"]);
    std::env::set_var("XDG_STATE_HOME", guard.path("state"));
    std::env::set_var("XDG_RUNTIME_DIR", guard.path("runtime"));
    std::env::set_var("CLAUDE_SESSION_ID", "synthesized-audit-session");

    let mut synthesized = base_binding();
    synthesized.synthesized_by_helper = true;
    let minted = mint_token(synthesized.clone(), 60).unwrap();
    let claims = decode_token_payload(&minted.file_path);
    assert_eq!(
        claims.get("synthesized_by_helper").and_then(Value::as_bool),
        Some(true)
    );

    let mut default_binding = synthesized;
    default_binding.synthesized_by_helper = false;
    assert!(verify_token(default_binding).valid);
}

#[test]
fn consent_binding_fixture_contract_stays_valid_for_future_bootstrap_synthesizer() {
    const FIXTURES: &[&str] = &[
        "deploy_create.pending.json",
        "apps_create.from_file.json",
        "apps_create.interactive.json",
        "env_set.json",
        "helper_synthesized.deploy_create.json",
    ];

    for fixture in FIXTURES {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/consent-bindings")
            .join(fixture);
        let raw = fs::read_to_string(&path).unwrap();
        let binding: ConsentBinding = serde_json::from_str(&raw).unwrap();
        validate_binding_schema(&binding).unwrap_or_else(|err| {
            panic!(
                "{} should be a valid consent binding fixture: {err}",
                path.display()
            )
        });

        if *fixture == "helper_synthesized.deploy_create.json" {
            assert!(binding.synthesized_by_helper);
        }
    }
}

#[test]
fn consent_binding_schema_accepts_known_actions_and_rejects_required_field_gaps() {
    let mut deploy = base_binding();
    assert!(validate_binding_schema(&deploy).is_ok());

    deploy.action = "unknown_publish".into();
    assert_eq!(
        validate_binding_schema(&deploy).unwrap_err().to_string(),
        "binding_schema:unknown_action:unknown_publish"
    );

    let mut missing_commit = base_binding();
    missing_commit.commit_sha.clear();
    assert_eq!(
        validate_binding_schema(&missing_commit)
            .unwrap_err()
            .to_string(),
        "binding_schema:missing_field:commit_sha"
    );

    let valid_cases = [
        (
            "apps_create",
            "",
            "",
            "",
            "",
            [("source", "apphub.yaml")].as_slice(),
        ),
        (
            "apps_create",
            "",
            "",
            "",
            "",
            [("source", "interactive")].as_slice(),
        ),
        (
            "apps_update",
            "paydrop",
            "",
            "",
            "",
            [
                ("slug", "paydrop"),
                ("fields", "name,visibility"),
                ("name", "Paydrop"),
                ("visibility", "private"),
            ]
            .as_slice(),
        ),
        (
            "apps_delete",
            "paydrop",
            "",
            "",
            "",
            [("slug", "paydrop")].as_slice(),
        ),
        (
            "env_set",
            "paydrop",
            "",
            "",
            "",
            [("key", "DATABASE_URL")].as_slice(),
        ),
        (
            "env_delete",
            "paydrop",
            "",
            "",
            "",
            [("key", "DATABASE_URL")].as_slice(),
        ),
        (
            "github_connect",
            "paydrop",
            "",
            "",
            "",
            [("repo", "paydrop"), ("branch", "main")].as_slice(),
        ),
        (
            "github_disconnect",
            "paydrop",
            "",
            "",
            "",
            [("slug", "paydrop")].as_slice(),
        ),
        (
            "deploy_cancel",
            "paydrop",
            "",
            "",
            "",
            [("deployment_id", "dep_123")].as_slice(),
        ),
        (
            "profile_add",
            "",
            "",
            "",
            "",
            [
                ("profile", "corp"),
                ("endpoint", "https://corp.example.test"),
            ]
            .as_slice(),
        ),
        (
            "profile_use",
            "",
            "",
            "",
            "",
            [("profile", "corp")].as_slice(),
        ),
        (
            "apis_call",
            "",
            "",
            "",
            "",
            [("endpoint_id", "endpoint_123"), ("method", "POST")].as_slice(),
        ),
        (
            "publish_submit",
            "paydrop",
            "",
            "",
            "",
            [
                ("note_length", "4"),
                (
                    "note_digest",
                    "sha256:e5d5b971139eefeb36d6edb9938fa246740c90da2003626487eb2d5d9646aec6",
                ),
            ]
            .as_slice(),
        ),
        (
            "deploy_rollback",
            "paydrop",
            "",
            "",
            "",
            [("from_deployment", "dep_123")].as_slice(),
        ),
        (
            "invitation_send",
            "",
            "",
            "",
            "",
            [("email", "user@example.com"), ("tenant", "acme")].as_slice(),
        ),
        (
            "invitation_bulk",
            "",
            "",
            "",
            "",
            [("source", "users.csv"), ("tenant", "acme")].as_slice(),
        ),
        (
            "invitation_cancel",
            "",
            "",
            "",
            "",
            [("invitation_id", "inv_123"), ("tenant", "acme")].as_slice(),
        ),
        (
            "invitation_resend",
            "",
            "",
            "",
            "",
            [("invitation_id", "inv_123"), ("tenant", "acme")].as_slice(),
        ),
        ("access_grant", "paydrop", "", "", "", [].as_slice()),
        ("access_revoke", "paydrop", "", "", "", [].as_slice()),
        (
            "access_invite",
            "paydrop",
            "",
            "",
            "",
            [("user", "user_123")].as_slice(),
        ),
        (
            "access_uninvite",
            "paydrop",
            "",
            "",
            "",
            [("user", "user_123")].as_slice(),
        ),
        (
            "tables_create",
            "paydrop",
            "",
            "",
            "",
            [("table", "orders"), ("column", "title:text")].as_slice(),
        ),
        (
            "tables_drop",
            "paydrop",
            "",
            "",
            "",
            [("table", "orders")].as_slice(),
        ),
        (
            "tables_columns_add",
            "paydrop",
            "",
            "",
            "",
            [("table", "orders"), ("name", "title"), ("type", "text")].as_slice(),
        ),
        (
            "tables_columns_remove",
            "paydrop",
            "",
            "",
            "",
            [("table", "orders"), ("name", "title")].as_slice(),
        ),
        (
            "tables_grants_issue",
            "paydrop",
            "",
            "",
            "",
            [
                ("table", "orders"),
                ("principal_id", "user_123"),
                ("actions", "read,write"),
            ]
            .as_slice(),
        ),
        (
            "tables_grants_revoke",
            "paydrop",
            "",
            "",
            "",
            [("table", "orders"), ("grant_id", "grant_123")].as_slice(),
        ),
        (
            "data_insert",
            "paydrop",
            "",
            "",
            "",
            [
                ("table", "orders"),
                ("source", "body"),
                (
                    "body_digest",
                    "sha256:3529d0a41555dcce5409451ca186444db8e972ff8d5867546e0e7928f37408f5",
                ),
            ]
            .as_slice(),
        ),
        (
            "data_update",
            "paydrop",
            "",
            "",
            "",
            [
                ("table", "orders"),
                ("row_id", "row_123"),
                ("source", "body"),
                (
                    "body_digest",
                    "sha256:a1a06dd13348c8b021ec36ab4c9cc7ec3bb9dfc979e57017429ee59ea908a277",
                ),
            ]
            .as_slice(),
        ),
        (
            "data_delete",
            "paydrop",
            "",
            "",
            "",
            [("table", "orders"), ("row_id", "row_123")].as_slice(),
        ),
        (
            "connector_create",
            "",
            "",
            "",
            "",
            [
                ("name", "warehouse"),
                ("source", "config_file"),
                ("tenant", "acme"),
                ("engine", "postgres"),
                ("config_file", "cfg.json"),
                ("config_digest", "sha256:cfg"),
                ("credentials_file", "creds.json"),
                ("credentials_digest", "sha256:creds"),
            ]
            .as_slice(),
        ),
        (
            "connector_update",
            "",
            "",
            "",
            "",
            [
                ("connector_id", "conn_123"),
                ("fields", "config,enabled"),
                ("source", "config_file"),
                ("tenant", "acme"),
                ("config_file", "cfg.json"),
                ("config_digest", "sha256:cfg"),
                ("enabled", "true"),
            ]
            .as_slice(),
        ),
        (
            "connector_credentials_set",
            "",
            "",
            "",
            "",
            [
                ("connector_id", "conn_123"),
                ("source", "credentials_file"),
                ("tenant", "acme"),
                ("credentials_file", "creds.json"),
                ("credentials_digest", "sha256:creds"),
            ]
            .as_slice(),
        ),
        (
            "connector_delete",
            "",
            "",
            "",
            "",
            [("connector_id", "conn_123"), ("tenant", "acme")].as_slice(),
        ),
        (
            "apps_fork",
            "paydrop",
            "",
            "",
            "",
            [
                ("source", "paydrop"),
                ("slug", "paydrop-copy"),
                ("subdomain", "paydrop-copy"),
                ("tenant", "acme"),
                ("name", "Paydrop"),
                ("template", "<source>"),
                ("repo_public", "false"),
            ]
            .as_slice(),
        ),
        ("apps_suspend", "paydrop", "", "", "", [].as_slice()),
        ("apps_resume", "paydrop", "", "", "", [].as_slice()),
        (
            "auth_oauth_client_create",
            "paydrop",
            "",
            "",
            "",
            [
                ("name", "web"),
                ("type", "confidential"),
                ("redirect_uris", "https://example.test/callback"),
                ("scopes", "openid,profile,email"),
                ("grant_types", "authorization_code"),
            ]
            .as_slice(),
        ),
        (
            "auth_oauth_revoke",
            "",
            "",
            "",
            "",
            [("target", "tok_123"), ("client_id", "default")].as_slice(),
        ),
        (
            "auth_oauth_consent_revoke",
            "",
            "",
            "",
            "",
            [("client_id", "client_123")].as_slice(),
        ),
        (
            "auth_pat_issue",
            "",
            "",
            "",
            "",
            [
                ("name", "ci-token"),
                ("expires_in_days", "90"),
                ("use", "false"),
                ("no_save", "false"),
                ("show_token", "false"),
            ]
            .as_slice(),
        ),
        (
            "auth_pat_revoke",
            "",
            "",
            "",
            "",
            [("pat_id", "pat_123")].as_slice(),
        ),
        (
            "auth_pat_use",
            "",
            "",
            "",
            "",
            [("pat_id", "pat_123"), ("profile", "default")].as_slice(),
        ),
        (
            "auth_pat_unset",
            "",
            "",
            "",
            "",
            [("target", "active_pat"), ("profile", "default")].as_slice(),
        ),
        (
            "auth_logout",
            "",
            "",
            "",
            "",
            [("profile", "default")].as_slice(),
        ),
        (
            "auth_pat_rotate",
            "",
            "",
            "",
            "",
            [("name", "rotated")].as_slice(),
        ),
        (
            "resource_namespace_create",
            "",
            "",
            "",
            "",
            [("name", "Finance"), ("tenant", "acme")].as_slice(),
        ),
        (
            "resource_rename",
            "",
            "",
            "",
            "",
            [
                ("resource_id", "res_123"),
                ("name", "Finance"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "resource_move",
            "",
            "",
            "",
            "",
            [
                ("resource_id", "res_123"),
                ("parent_id", "root"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "resource_bulk_register",
            "",
            "",
            "",
            "",
            [
                ("connector_id", "conn_123"),
                ("source", "items_file"),
                ("tenant", "acme"),
                ("items_file", "items.json"),
                ("items_digest", "sha256:items"),
            ]
            .as_slice(),
        ),
        (
            "resource_delete",
            "",
            "",
            "",
            "",
            [("resource_id", "res_123"), ("tenant", "acme")].as_slice(),
        ),
        (
            "resource_tag_attach",
            "",
            "",
            "",
            "",
            [
                ("resource_id", "res_123"),
                ("tag_id", "tag_123"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "resource_tag_detach",
            "",
            "",
            "",
            "",
            [
                ("resource_id", "res_123"),
                ("tag_id", "tag_123"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        ("update_apply", "paydrop", "", "", "", [].as_slice()),
        ("deploy_logs_kill", "paydrop", "", "", "", [].as_slice()),
        ("auth_login", "_", "default", "_", "_", [].as_slice()),
    ];

    for (action, app_id, profile, branch, commit_sha, context) in valid_cases {
        let mut binding = ConsentBinding {
            tool_call_id: "sess:tool".into(),
            action: action.into(),
            app_id: app_id.into(),
            profile: profile.into(),
            branch: branch.into(),
            commit_sha: commit_sha.into(),
            context: HashMap::new(),
            synthesized_by_helper: false,
        };
        for (key, value) in context {
            binding.context.insert((*key).into(), (*value).into());
        }
        assert!(validate_binding_schema(&binding).is_ok(), "{action}");
    }

    let mut missing_source = ConsentBinding {
        tool_call_id: "sess:tool".into(),
        action: "apps_create".into(),
        app_id: "".into(),
        profile: "".into(),
        branch: "".into(),
        commit_sha: "".into(),
        context: HashMap::new(),
        synthesized_by_helper: false,
    };
    missing_source
        .context
        .insert("slug".into(), "paydrop".into());
    assert_eq!(
        validate_binding_schema(&missing_source)
            .unwrap_err()
            .to_string(),
        "binding_schema:missing_context:source"
    );
}

#[test]
fn consent_parser_recognizes_nested_shell_destructive_intents_and_ignores_safe_commands() {
    let update = parse_axhub_command(
        r#"sh -c 'echo before && axhub update apply --app=paydrop --profile=prod'"#,
    );
    assert!(update.is_destructive);
    assert_eq!(update.action.as_deref(), Some("update_apply"));
    assert_eq!(update.app_id.as_deref(), Some("paydrop"));
    assert_eq!(update.profile.as_deref(), Some("prod"));

    let kill = parse_axhub_command(
        r#"printf ok; (AXHUB_PROFILE=prod axhub deploy logs --app paydrop --kill)"#,
    );
    assert!(kill.is_destructive);
    assert_eq!(kill.action.as_deref(), Some("deploy_logs_kill"));
    assert_eq!(kill.app_id.as_deref(), Some("paydrop"));

    let login = parse_axhub_command(r#"eval "axhub auth login --profile dev""#);
    assert!(login.is_destructive);
    assert_eq!(login.action.as_deref(), Some("auth_login"));
    assert_eq!(login.profile.as_deref(), Some("dev"));

    for shell in [
        r#"bash -lc 'axhub apps delete paydrop --execute --json'"#,
        r#"sh -lc 'axhub apps delete paydrop --execute --json'"#,
        r#"sh -e -c 'axhub apps delete paydrop --execute --json'"#,
        r#"zsh -lc 'axhub apps delete paydrop --execute --json'"#,
    ] {
        let wrapped = parse_axhub_command(shell);
        assert!(wrapped.is_destructive, "{shell}");
        assert_eq!(wrapped.action.as_deref(), Some("apps_delete"), "{shell}");
        assert_eq!(wrapped.app_id.as_deref(), Some("paydrop"), "{shell}");
    }

    for wrapped in [
        "/opt/homebrew/bin/axhub apps delete paydrop --execute --json",
        "./axhub apps delete paydrop --execute --json",
        "env -S 'axhub apps delete paydrop --execute --json'",
        "env --split-string='axhub apps delete paydrop --execute --json'",
    ] {
        let parsed = parse_axhub_command(wrapped);
        assert!(parsed.is_destructive, "{wrapped}");
        assert_eq!(parsed.action.as_deref(), Some("apps_delete"), "{wrapped}");
        assert_eq!(parsed.app_id.as_deref(), Some("paydrop"), "{wrapped}");
    }

    let dynamic_eval =
        parse_axhub_command("eval $(echo axhub apps delete paydrop --execute --json)");
    assert!(dynamic_eval.is_destructive);
    assert_eq!(
        dynamic_eval.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        dynamic_eval.context.get("reason").map(String::as_str),
        Some("dynamic_shell_axhub_mutation")
    );

    for variable_command in [
        "AXHUB_BIN=axhub; $AXHUB_BIN apps delete paydrop --execute --json",
        "${AXHUB_BIN:-axhub} apps delete paydrop --execute --json",
        "CMD=axhub; $CMD apps delete paydrop --execute --json",
    ] {
        let parsed = parse_axhub_command(variable_command);
        assert!(parsed.is_destructive, "{variable_command}");
        assert_eq!(
            parsed.action.as_deref(),
            Some("unknown_axhub_mutation"),
            "{variable_command}"
        );
        assert_eq!(
            parsed.context.get("reason").map(String::as_str),
            Some("variable_axhub_command"),
            "{variable_command}"
        );
    }

    for runner_command in [
        "printf '%s\n' paydrop | xargs -I{} axhub apps delete {} --execute --json",
        "find . -name apphub.yaml -exec axhub apps delete paydrop --execute --json ;",
    ] {
        let parsed = parse_axhub_command(runner_command);
        assert!(parsed.is_destructive, "{runner_command}");
        assert_eq!(
            parsed.action.as_deref(),
            Some("unknown_axhub_mutation"),
            "{runner_command}"
        );
        assert_eq!(
            parsed.context.get("reason").map(String::as_str),
            Some("indirect_axhub_runner"),
            "{runner_command}"
        );
    }

    let stdin_shell =
        parse_axhub_command("printf '%s\n' 'axhub apps delete paydrop --execute --json' | sh");
    assert!(stdin_shell.is_destructive);
    assert_eq!(
        stdin_shell.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        stdin_shell.context.get("reason").map(String::as_str),
        Some("shell_script_axhub_mutation")
    );

    let shell_function =
        parse_axhub_command(r#"a(){ axhub "$@"; }; a apps delete paydrop --execute --json"#);
    assert!(shell_function.is_destructive);
    assert_eq!(
        shell_function.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        shell_function.context.get("reason").map(String::as_str),
        Some("shell_alias_axhub_mutation")
    );

    for shell_alias in [
        "alias a=axhub; a apps delete paydrop --execute --json",
        "shopt -s expand_aliases; alias a=axhub; a apps delete paydrop --execute --json",
        "alias a=/opt/homebrew/bin/axhub; a apps delete paydrop --execute --json",
        "a ()\n{ axhub \"$@\"; }\na apps delete paydrop --execute --json",
    ] {
        let parsed = parse_axhub_command(shell_alias);
        assert!(parsed.is_destructive, "{shell_alias}");
        assert_eq!(
            parsed.action.as_deref(),
            Some("unknown_axhub_mutation"),
            "{shell_alias}"
        );
        assert_eq!(
            parsed.context.get("reason").map(String::as_str),
            Some("shell_alias_axhub_mutation"),
            "{shell_alias}"
        );
    }

    let deploy_with_dynamic_arg = parse_axhub_command(
        "axhub deploy create --app paydrop --commit $(git rev-parse HEAD) --execute --json",
    );
    assert!(deploy_with_dynamic_arg.is_destructive);
    assert_eq!(
        deploy_with_dynamic_arg.action.as_deref(),
        Some("deploy_create")
    );

    let safe = parse_axhub_command("axhub deploy logs --app paydrop");
    assert!(!safe.is_destructive);
    assert!(safe.action.is_none());

    let chained = parse_axhub_commands(
        "axhub deploy create --app paydrop --commit abc123 --execute --json && axhub apps delete victim --execute --json",
    );
    assert_eq!(chained.len(), 2);
    assert_eq!(chained[0].action.as_deref(), Some("deploy_create"));
    assert_eq!(chained[1].action.as_deref(), Some("apps_delete"));

    let hidden_wrapped = parse_axhub_commands(
        "axhub deploy create --app paydrop --commit abc123 --execute --json && bash -lc 'axhub apps delete victim --execute --json'",
    );
    assert_eq!(hidden_wrapped.len(), 2);
    assert_eq!(hidden_wrapped[0].action.as_deref(), Some("deploy_create"));
    assert_eq!(hidden_wrapped[1].action.as_deref(), Some("apps_delete"));
    assert_eq!(hidden_wrapped[1].app_id.as_deref(), Some("victim"));

    let hidden_path_qualified = parse_axhub_commands(
        "axhub deploy create --app paydrop --commit abc123 --execute --json && /opt/homebrew/bin/axhub apps delete victim --execute --json",
    );
    assert_eq!(hidden_path_qualified.len(), 2);
    assert_eq!(
        hidden_path_qualified[1].action.as_deref(),
        Some("apps_delete")
    );
    assert_eq!(hidden_path_qualified[1].app_id.as_deref(), Some("victim"));

    let repeated = parse_axhub_commands(
        "axhub apps delete paydrop --execute --json ; axhub apps delete paydrop --execute --json",
    );
    assert_eq!(
        repeated.len(),
        2,
        "identical destructive commands must not collapse into one consent check"
    );

    let loop_wrapped =
        parse_axhub_command("for i in 1; do axhub apps delete paydrop --execute --json; done");
    assert!(loop_wrapped.is_destructive);
    assert_eq!(loop_wrapped.action.as_deref(), Some("apps_delete"));
    assert_eq!(loop_wrapped.app_id.as_deref(), Some("paydrop"));

    let newline_loop_wrapped =
        parse_axhub_command("for i in 1\ndo axhub apps delete paydrop --execute --json\ndone");
    assert!(newline_loop_wrapped.is_destructive);
    assert_eq!(newline_loop_wrapped.action.as_deref(), Some("apps_delete"));
    assert_eq!(newline_loop_wrapped.app_id.as_deref(), Some("paydrop"));

    let env_wrapped =
        parse_axhub_command("env AXHUB_PROFILE=prod axhub apps delete paydrop --execute --json");
    assert!(env_wrapped.is_destructive);
    assert_eq!(env_wrapped.action.as_deref(), Some("apps_delete"));
    assert_eq!(env_wrapped.app_id.as_deref(), Some("paydrop"));

    let sudo_wrapped =
        parse_axhub_command("sudo -u deploy axhub apps delete paydrop --execute --json");
    assert!(sudo_wrapped.is_destructive);
    assert_eq!(sudo_wrapped.action.as_deref(), Some("apps_delete"));
    assert_eq!(sudo_wrapped.app_id.as_deref(), Some("paydrop"));

    let nohup_wrapped = parse_axhub_command("nohup axhub apps delete paydrop --execute --json");
    assert!(nohup_wrapped.is_destructive);
    assert_eq!(nohup_wrapped.action.as_deref(), Some("apps_delete"));
    assert_eq!(nohup_wrapped.app_id.as_deref(), Some("paydrop"));

    for wrapped in [
        "time -p axhub apps delete paydrop --execute --json",
        "command -p axhub apps delete paydrop --execute --json",
        "exec -a axhub-proxy axhub apps delete paydrop --execute --json",
        "nohup -- axhub apps delete paydrop --execute --json",
    ] {
        let parsed = parse_axhub_command(wrapped);
        assert!(parsed.is_destructive, "{wrapped}");
        assert_eq!(parsed.action.as_deref(), Some("apps_delete"), "{wrapped}");
        assert_eq!(parsed.app_id.as_deref(), Some("paydrop"), "{wrapped}");
    }

    let unknown_fleet =
        parse_axhub_command("axhub deploy fleet --apps paydrop,crm --commit abc --execute --json");
    assert!(unknown_fleet.is_destructive);
    assert_eq!(
        unknown_fleet.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        unknown_fleet
            .context
            .get("command_path")
            .map(String::as_str),
        Some("deploy fleet")
    );

    let env_profile_logout = parse_axhub_command("AXHUB_PROFILE=prod axhub auth logout --json");
    assert!(env_profile_logout.is_destructive);
    assert_eq!(env_profile_logout.action.as_deref(), Some("auth_logout"));
    assert_eq!(
        env_profile_logout
            .context
            .get("profile")
            .map(String::as_str),
        Some("prod")
    );

    let publish_note = parse_axhub_command(
        r#"axhub publish --app paydrop --note "ship totally different note" --json"#,
    );
    assert_eq!(publish_note.action.as_deref(), Some("publish_submit"));
    assert_eq!(
        publish_note.context.get("note_length").map(String::as_str),
        Some("27"),
        "quoted multi-word publish note must stay intact"
    );

    let app_update = parse_axhub_command(
        r#"axhub apps update paydrop --description "launch copy update" --json"#,
    );
    assert_eq!(app_update.action.as_deref(), Some("apps_update"));
    assert_eq!(
        app_update.context.get("description").map(String::as_str),
        Some("launch copy update"),
        "quoted multi-word app update value must stay intact"
    );

    let temp = tempfile::tempdir().unwrap();
    let axhub_payload_dir = temp.path().join("axhub");
    std::fs::create_dir_all(&axhub_payload_dir).unwrap();
    let cfg = axhub_payload_dir.join("cfg.json").display().to_string();
    let creds = temp.path().join("creds.json").display().to_string();
    std::fs::write(&cfg, br#"{"host":"db.internal"}"#).unwrap();
    std::fs::write(&creds, br#"{"username":"svc","password":"redacted"}"#).unwrap();
    let same_bash_file_write = parse_axhub_command(&format!(
        "printf '{{\"host\":\"evil\"}}' > \"{cfg}\" && axhub connectors create --tenant acme --name warehouse --engine postgres --config-file \"{cfg}\" --credentials-file \"{creds}\" --execute --json"
    ));
    assert!(same_bash_file_write.is_destructive);
    assert_eq!(
        same_bash_file_write.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        same_bash_file_write
            .context
            .get("reason")
            .map(String::as_str),
        Some("same_bash_payload_file_write")
    );

    let python_payload_rewrite = parse_axhub_command(&format!(
        "python3 -c 'from pathlib import Path; Path(\"{cfg}\").write_text(\"{{}}\")' && axhub connectors create --tenant acme --name warehouse --engine postgres --config-file \"{cfg}\" --credentials-file \"{creds}\" --execute --json"
    ));
    assert!(python_payload_rewrite.is_destructive);
    assert_eq!(
        python_payload_rewrite.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        python_payload_rewrite
            .context
            .get("reason")
            .map(String::as_str),
        Some("same_bash_payload_file_write")
    );

    let parent = axhub_payload_dir.display().to_string();
    let relative_payload_rewrite = parse_axhub_command(&format!(
        "cd \"{parent}\" && printf '{{\"host\":\"evil\"}}' > cfg.json && axhub connectors create --tenant acme --name warehouse --engine postgres --config-file \"{cfg}\" --credentials-file \"{creds}\" --execute --json"
    ));
    assert!(relative_payload_rewrite.is_destructive);
    assert_eq!(
        relative_payload_rewrite.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        relative_payload_rewrite
            .context
            .get("reason")
            .map(String::as_str),
        Some("same_bash_payload_file_write")
    );

    let link_payload_swap = parse_axhub_command(&format!(
        "ln -sf \"{creds}\" \"{cfg}\" && axhub connectors create --tenant acme --name warehouse --engine postgres --config-file \"{cfg}\" --credentials-file \"{creds}\" --execute --json"
    ));
    assert!(link_payload_swap.is_destructive);
    assert_eq!(
        link_payload_swap.action.as_deref(),
        Some("unknown_axhub_mutation")
    );
    assert_eq!(
        link_payload_swap.context.get("reason").map(String::as_str),
        Some("same_bash_payload_file_write")
    );
}

#[test]
fn consent_parser_recognizes_current_cli_mutation_actions_with_stable_context() {
    let cases = [
        (
            "axhub env set DATABASE_URL --app paydrop --from-stdin --json",
            "env_set",
            Some("paydrop"),
            [("key", "DATABASE_URL")].as_slice(),
        ),
        (
            "axhub env delete DATABASE_URL --app paydrop --force --confirm=DATABASE_URL --json",
            "env_delete",
            Some("paydrop"),
            [("key", "DATABASE_URL")].as_slice(),
        ),
        (
            "axhub apps create --from-file apphub.yaml --yes --json",
            "apps_create",
            None,
            [("source", "apphub.yaml")].as_slice(),
        ),
        (
            "axhub apps create --interactive --json",
            "apps_create",
            None,
            [("source", "interactive")].as_slice(),
        ),
        (
            "axhub apps create --name Paydrop --slug paydrop --json",
            "apps_create",
            Some("paydrop"),
            [("slug", "paydrop"), ("source", "inline")].as_slice(),
        ),
        (
            "axhub apps update paydrop --name Paydrop --visibility private --json",
            "apps_update",
            Some("paydrop"),
            [("slug", "paydrop"), ("fields", "name,visibility"), ("name", "Paydrop"), ("visibility", "private")].as_slice(),
        ),
        (
            "axhub apps delete paydrop --yes --json",
            "apps_delete",
            Some("paydrop"),
            [("slug", "paydrop")].as_slice(),
        ),
        (
            "axhub apps delete 165 --yes --json",
            "apps_delete",
            Some("165"),
            [("slug", "165")].as_slice(),
        ),
        (
            "axhub apps delete paydrop --dry-run --json",
            "apps_delete",
            Some("paydrop"),
            [("slug", "paydrop")].as_slice(),
        ),
        (
            "axhub github connect paydrop --account jocoding --repo paydrop --branch main --json",
            "github_connect",
            Some("paydrop"),
            [("repo", "paydrop"), ("branch", "main")].as_slice(),
        ),
        (
            "axhub github disconnect paydrop --force --confirm=paydrop --json",
            "github_disconnect",
            Some("paydrop"),
            [("slug", "paydrop")].as_slice(),
        ),
        (
            "axhub deploy cancel dep_123 --app paydrop --json",
            "deploy_cancel",
            Some("paydrop"),
            [("deployment_id", "dep_123")].as_slice(),
        ),
        (
            "axhub deploy rollback --app paydrop --from-deployment dep_123 --execute --json",
            "deploy_rollback",
            Some("paydrop"),
            [("from_deployment", "dep_123")].as_slice(),
        ),
        (
            "axhub deploy create --app paydrop --commit abc123 --dry-run --execute --json",
            "deploy_create",
            Some("paydrop"),
            [].as_slice(),
        ),
        (
            "axhub publish --app paydrop --note ship --json",
            "publish_submit",
            Some("paydrop"),
            [
                ("note_length", "4"),
                (
                    "note_digest",
                    "sha256:e5d5b971139eefeb36d6edb9938fa246740c90da2003626487eb2d5d9646aec6",
                ),
            ]
            .as_slice(),
        ),
        (
            "axhub invitations send user@example.com --role member --tenant acme --json",
            "invitation_send",
            None,
            [
                ("email", "user@example.com"),
                ("role", "member"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "axhub invitations bulk --from-file users.csv --role member --strict --execute --tenant acme --json",
            "invitation_bulk",
            None,
            [("source", "users.csv"), ("role", "member"), ("tenant", "acme")].as_slice(),
        ),
        (
            "axhub invitations cancel inv_123 --execute --tenant acme --json",
            "invitation_cancel",
            None,
            [("invitation_id", "inv_123"), ("tenant", "acme")].as_slice(),
        ),
        (
            "axhub invitations resend inv_123 --role member --execute --tenant acme --json",
            "invitation_resend",
            None,
            [
                ("invitation_id", "inv_123"),
                ("role", "member"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "axhub access grant --app paydrop --json",
            "access_grant",
            Some("paydrop"),
            [].as_slice(),
        ),
        (
            "axhub access revoke --app paydrop --execute --json",
            "access_revoke",
            Some("paydrop"),
            [].as_slice(),
        ),
        (
            "axhub access invite --app paydrop --user user_123 --execute --json",
            "access_invite",
            Some("paydrop"),
            [("user", "user_123")].as_slice(),
        ),
        (
            "axhub access uninvite --app paydrop --user user_123 --execute --json",
            "access_uninvite",
            Some("paydrop"),
            [("user", "user_123")].as_slice(),
        ),
        (
            "axhub tables create orders --app paydrop --column title:text --owner-column owner_id --execute --json",
            "tables_create",
            Some("paydrop"),
            [("table", "orders"), ("column", "title:text")].as_slice(),
        ),
        (
            "axhub tables drop orders --app paydrop --confirm orders --execute --json",
            "tables_drop",
            Some("paydrop"),
            [("table", "orders")].as_slice(),
        ),
        (
            "axhub tables columns add orders --app paydrop --name title --type text --nullable --execute --json",
            "tables_columns_add",
            Some("paydrop"),
            [("table", "orders"), ("name", "title"), ("type", "text")].as_slice(),
        ),
        (
            "axhub tables columns remove orders --app paydrop --name title --execute --json",
            "tables_columns_remove",
            Some("paydrop"),
            [("table", "orders"), ("name", "title")].as_slice(),
        ),
        (
            "axhub tables grants issue orders --app paydrop --principal-id user_123 --principal-type user --actions read,write --execute --json",
            "tables_grants_issue",
            Some("paydrop"),
            [
                ("table", "orders"),
                ("principal_id", "user_123"),
                ("actions", "read,write"),
            ]
            .as_slice(),
        ),
        (
            "axhub tables grants issue --app paydrop --table orders --principal-id user_123 --principal-type user --actions read,write --execute --json",
            "tables_grants_issue",
            Some("paydrop"),
            [
                ("table", "orders"),
                ("principal_id", "user_123"),
                ("actions", "read,write"),
            ]
            .as_slice(),
        ),
        (
            "axhub tables grants revoke --app paydrop --table orders --grant-id grant_123 --execute --json",
            "tables_grants_revoke",
            Some("paydrop"),
            [("table", "orders"), ("grant_id", "grant_123")].as_slice(),
        ),
        (
            "axhub data insert orders --app paydrop --body '{\"title\":\"A\"}' --execute --json",
            "data_insert",
            Some("paydrop"),
            [
                ("table", "orders"),
                ("source", "body"),
                (
                    "body_digest",
                    "sha256:3529d0a41555dcce5409451ca186444db8e972ff8d5867546e0e7928f37408f5",
                ),
            ]
            .as_slice(),
        ),
        (
            "axhub data update orders row_123 --app paydrop --body '{\"title\":\"B\"}' --execute --json",
            "data_update",
            Some("paydrop"),
            [
                ("table", "orders"),
                ("row_id", "row_123"),
                ("source", "body"),
                (
                    "body_digest",
                    "sha256:a1a06dd13348c8b021ec36ab4c9cc7ec3bb9dfc979e57017429ee59ea908a277",
                ),
            ]
            .as_slice(),
        ),
        (
            "axhub data delete orders row_123 --app paydrop --execute --json",
            "data_delete",
            Some("paydrop"),
            [("table", "orders"), ("row_id", "row_123")].as_slice(),
        ),
        (
            "axhub connectors create --tenant acme --name warehouse --engine postgres --config-file cfg.json --credentials-stdin --execute --json",
            "connector_create",
            None,
            [
                ("tenant", "acme"),
                ("name", "warehouse"),
                ("source", "config_file"),
                ("engine", "postgres"),
                ("config_file", "cfg.json"),
                ("credentials_source", "stdin"),
            ]
            .as_slice(),
        ),
        (
            "axhub connectors update conn_123 --tenant acme --config-file cfg.json --enabled --execute --json",
            "connector_update",
            None,
            [
                ("connector_id", "conn_123"),
                ("tenant", "acme"),
                ("fields", "config,enabled"),
                ("source", "config_file"),
                ("config_file", "cfg.json"),
                ("enabled", "true"),
            ]
            .as_slice(),
        ),
        (
            "axhub connectors credentials-set conn_123 --tenant acme --credentials-stdin --execute --json",
            "connector_credentials_set",
            None,
            [
                ("connector_id", "conn_123"),
                ("tenant", "acme"),
                ("source", "stdin"),
                ("credentials_source", "stdin"),
            ]
            .as_slice(),
        ),
        (
            "axhub connectors delete conn_123 --tenant acme --execute --json",
            "connector_delete",
            None,
            [("connector_id", "conn_123"), ("tenant", "acme")].as_slice(),
        ),
        (
            "axhub apps fork paydrop --slug paydrop-copy --subdomain paydrop-copy --name Paydrop --tenant acme --execute --json",
            "apps_fork",
            Some("paydrop"),
            [
                ("source", "paydrop"),
                ("slug", "paydrop-copy"),
                ("subdomain", "paydrop-copy"),
                ("tenant", "acme"),
                ("name", "Paydrop"),
                ("template", "<source>"),
                ("repo_public", "false"),
            ]
            .as_slice(),
        ),
        (
            "axhub apps fork paydrop --slug paydrop-copy --subdomain paydrop-copy --repo-public --template tmpl_1 --execute --json",
            "apps_fork",
            Some("paydrop"),
            [
                ("source", "paydrop"),
                ("slug", "paydrop-copy"),
                ("subdomain", "paydrop-copy"),
                ("tenant", "<active>"),
                ("name", "paydrop-copy"),
                ("template", "tmpl_1"),
                ("repo_public", "true"),
            ]
            .as_slice(),
        ),
        (
            "axhub apps suspend paydrop --execute --json",
            "apps_suspend",
            Some("paydrop"),
            [].as_slice(),
        ),
        (
            "axhub apps resume paydrop --execute --json",
            "apps_resume",
            Some("paydrop"),
            [].as_slice(),
        ),
        (
            "axhub resources namespace create --tenant acme --name Finance --parent-id root --execute --json",
            "resource_namespace_create",
            None,
            [
                ("name", "Finance"),
                ("tenant", "acme"),
                ("parent_id", "root"),
            ]
            .as_slice(),
        ),
        (
            "axhub resources rename res_123 --tenant acme --name Finance --execute --json",
            "resource_rename",
            None,
            [
                ("resource_id", "res_123"),
                ("name", "Finance"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "axhub resources move res_123 --tenant acme --root --execute --json",
            "resource_move",
            None,
            [
                ("resource_id", "res_123"),
                ("parent_id", "root"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "axhub resources bulk-register --tenant acme --connector-id conn_123 --items-file items.json --include-columns --execute --json",
            "resource_bulk_register",
            None,
            [
                ("connector_id", "conn_123"),
                ("source", "items_file"),
                ("tenant", "acme"),
                ("items_file", "items.json"),
            ]
            .as_slice(),
        ),
        (
            "axhub resources bulk-register --tenant acme --connector-id conn_123 --items-json '[{\"id\":\"r1\"}]' --execute --json",
            "resource_bulk_register",
            None,
            [
                ("connector_id", "conn_123"),
                ("source", "items_json"),
                ("tenant", "acme"),
                (
                    "items_digest",
                    "sha256:a10d3b7f0554b0797e1c7dd145507f71a2aecb2b2a3cc446d9a4642463978b28",
                ),
            ]
            .as_slice(),
        ),
        (
            "axhub auth oauth client create --app paydrop --name web --type confidential --redirect-uri https://example.test/callback --scope openid --scope profile --scope email --grant-type authorization_code --execute --json",
            "auth_oauth_client_create",
            Some("paydrop"),
            [
                ("name", "web"),
                ("type", "confidential"),
                ("redirect_uris", "https://example.test/callback"),
                ("scopes", "openid,profile,email"),
                ("grant_types", "authorization_code"),
            ]
            .as_slice(),
        ),
        (
            "axhub auth oauth revoke tok_123 --client-id client_123 --token-type-hint refresh_token --execute --json",
            "auth_oauth_revoke",
            None,
            [
                ("target", "tok_123"),
                ("client_id", "client_123"),
                ("token_type_hint", "refresh_token"),
            ]
            .as_slice(),
        ),
        (
            "axhub auth oauth consent revoke client_123 --execute --json",
            "auth_oauth_consent_revoke",
            None,
            [("client_id", "client_123")].as_slice(),
        ),
        (
            "axhub auth pat issue --name ci-token --expires-in-days 90 --json",
            "auth_pat_issue",
            None,
            [
                ("name", "ci-token"),
                ("expires_in_days", "90"),
                ("use", "false"),
                ("no_save", "false"),
                ("show_token", "false"),
            ]
            .as_slice(),
        ),
        (
            "axhub auth pat revoke pat_123 --execute --json",
            "auth_pat_revoke",
            None,
            [("pat_id", "pat_123")].as_slice(),
        ),
        (
            "axhub auth pat use pat_123 --profile default --json",
            "auth_pat_use",
            None,
            [("pat_id", "pat_123"), ("profile", "default")].as_slice(),
        ),
        (
            "axhub auth pat unset --profile default --json",
            "auth_pat_unset",
            None,
            [("target", "active_pat"), ("profile", "default")].as_slice(),
        ),
        (
            "axhub auth logout --profile default --json",
            "auth_logout",
            None,
            [("profile", "default")].as_slice(),
        ),
        (
            "axhub auth pat rotate --name rotated --expires-in-days 90 --json",
            "auth_pat_rotate",
            None,
            [("name", "rotated"), ("expires_in_days", "90")].as_slice(),
        ),
        (
            "axhub resources delete res_123 --tenant acme --cascade --execute --json",
            "resource_delete",
            None,
            [("resource_id", "res_123"), ("tenant", "acme")].as_slice(),
        ),
        (
            "axhub resources tag-attach res_123 --tenant acme --tag-id tag_123 --execute --json",
            "resource_tag_attach",
            None,
            [
                ("resource_id", "res_123"),
                ("tag_id", "tag_123"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "axhub resources tag-detach res_123 --tenant acme --tag-id tag_123 --execute --json",
            "resource_tag_detach",
            None,
            [
                ("resource_id", "res_123"),
                ("tag_id", "tag_123"),
                ("tenant", "acme"),
            ]
            .as_slice(),
        ),
        (
            "axhub profile add corp --endpoint https://corp.example.test --json",
            "profile_add",
            None,
            [
                ("profile", "corp"),
                ("endpoint", "https://corp.example.test"),
            ]
            .as_slice(),
        ),
        (
            "axhub profile use corp --json",
            "profile_use",
            None,
            [("profile", "corp")].as_slice(),
        ),
        (
            "axhub apis call endpoint_123 --method POST --body-file payload.json --json",
            "apis_call",
            None,
            [
                ("endpoint_id", "endpoint_123"),
                ("method", "POST"),
                ("body_file", "payload.json"),
            ]
            .as_slice(),
        ),
    ];

    for (command, action, app_id, expected_context) in cases {
        let parsed = parse_axhub_command(command);
        assert!(parsed.is_destructive, "{command}");
        assert_eq!(parsed.action.as_deref(), Some(action), "{command}");
        assert_eq!(parsed.app_id.as_deref(), app_id, "{command}");
        for (key, value) in expected_context {
            assert_eq!(
                parsed.context.get(*key).map(String::as_str),
                Some(*value),
                "{command}"
            );
        }
    }

    for command in [
        "axhub apps create --help",
        "axhub apps create -h",
        "axhub deploy create --help",
        "axhub deploy create --app paydrop --commit abc123 --dry-run --json",
        r#"AXHUB_STDERR_TMP=$(mktemp)
AXHUB_STDOUT_TMP=$(mktemp)
APP_ID="paydrop"
COMMIT_SHA="abc123"
axhub deploy create --app "$APP_ID" --commit "$COMMIT_SHA" --dry-run --json >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP""#,
        r#"PROFILE_FLAG=()
PROFILE="paydrop"
if [ -n "${PROFILE:-}" ]; then
  PROFILE_FLAG=(--profile "$PROFILE")
fi
AXHUB_STDERR_TMP=$(mktemp)
AXHUB_STDOUT_TMP=$(mktemp)
axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --dry-run --json >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP""#,
        r#"AXHUB="/tmp/axhub-fixture/bin/axhub"
APP_ID="app_paydrop"
COMMIT_SHA="abc123"
PROFILE="paydrop"
"$AXHUB" deploy create --app "$APP_ID" --profile "$PROFILE" --commit "$COMMIT_SHA" --dry-run --json 2>&1"#,
        r#"CLAUDE_PLUGIN_ROOT="/tmp/axhub"
echo "CLAUDE_PLUGIN_ROOT=${CLAUDE_PLUGIN_ROOT:-<unset>}"
HELPER="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"
"$HELPER" route-decision --user-utterance "paydrop 배포해" --explicit"#,
        r#"if [ -z "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  if HELPER_FROM_PATH="$(command -v axhub-helpers 2>/dev/null)"; then
    export CLAUDE_PLUGIN_ROOT="$(cd "$(dirname "$HELPER_FROM_PATH")/.." && pwd)"
  fi
fi
echo "CLAUDE_PLUGIN_ROOT=${CLAUDE_PLUGIN_ROOT:-<unset>}"
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
"$HELPER" route-decision --user-utterance "paydrop 배포해" --explicit"#,
        r#"HELPER="/tmp/axhub/bin/axhub-helpers"; echo "CLAUDE_PLUGIN_ROOT=<not set>"; "$HELPER" route-decision --user-utterance "paydrop 배포" --explicit"#,
        "axhub bootstrap install-node",
        "axhub install-deps",
        "axhub admin setup team",
    ] {
        let parsed = parse_axhub_command(command);
        assert!(!parsed.is_destructive, "{command}");
        assert!(parsed.action.is_none(), "{command}");
    }
}

#[test]
fn bootstrap_synthesized_bindings_roundtrip_through_preauth_parser() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&[
        "XDG_STATE_HOME",
        "XDG_RUNTIME_DIR",
        "CLAUDE_SESSION_ID",
        "AXHUB_PROFILE",
    ]);
    std::env::set_var("XDG_STATE_HOME", guard.path("state"));
    std::env::set_var("XDG_RUNTIME_DIR", guard.path("runtime"));
    std::env::set_var("CLAUDE_SESSION_ID", "bootstrap-roundtrip-session");
    std::env::set_var("AXHUB_PROFILE", "prod");

    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("apphub.yaml"), "name: paydrop\n").unwrap();
    let _cwd = CwdGuard::enter(project.path());

    let apps_plan = run_bootstrap(&["--auto-chain".into(), "--json".into()], None);
    assert_eq!(apps_plan.exit_code, 0);
    assert_eq!(
        apps_plan.output.state,
        BootstrapState::ConsentRequiredAppsCreate
    );
    let apps_command = apps_plan.output.command.clone().unwrap();
    let apps_binding = apps_plan.output.consent_binding.clone().unwrap();
    let parsed_apps = parse_axhub_command(&apps_command.join(" "));
    assert_eq!(parsed_apps.action.as_deref(), Some("apps_create"));
    assert_eq!(apps_binding.context, parsed_apps.context);
    assert_eq!(apps_binding.app_id, parsed_apps.app_id.unwrap_or_default());

    mint_token(apps_binding.clone(), 60).unwrap();
    let mut actual_apps = apps_binding.clone();
    actual_apps.tool_call_id = "actual-session:toolu_apps".into();
    actual_apps.synthesized_by_helper = false;
    assert!(verify_or_claim_token(actual_apps).valid);

    let apps_record = json!({
        "schema_version": BOOTSTRAP_RECORD_SCHEMA_VERSION,
        "pending_action_id": apps_plan.output.pending_action_id.as_ref().unwrap(),
        "pending_action_hash": apps_plan.output.pending_action_hash.as_ref().unwrap(),
        "command_argv": apps_command,
        "exit_code": 0,
        "stdout_json": {
            "id": 42,
            "slug": "paydrop",
            "subdomain": "paydrop",
            "domain_id": 1
        },
        "stderr": ""
    });
    let recorded = run_bootstrap(
        &["--record".into(), "apps_create".into(), "--json".into()],
        Some(&apps_record.to_string()),
    );
    assert_eq!(recorded.exit_code, 0);

    std::process::Command::new("git")
        .args(["init", "-q"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "s4@example.invalid"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "S4 Test"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .status()
        .unwrap();

    let deploy_plan = run_bootstrap(&["--auto-chain".into(), "--json".into()], None);
    assert_eq!(deploy_plan.exit_code, 0);
    assert_eq!(
        deploy_plan.output.state,
        BootstrapState::ConsentRequiredDeployCreate
    );
    let deploy_command = deploy_plan.output.command.clone().unwrap();
    let deploy_binding = deploy_plan.output.consent_binding.clone().unwrap();
    assert!(
        !deploy_command.iter().any(|arg| arg == "--branch"),
        "{deploy_command:?}"
    );
    assert!(
        deploy_command.iter().any(|arg| arg == "--execute"),
        "{deploy_command:?}"
    );
    let parsed_deploy = parse_axhub_command(&deploy_command.join(" "));
    assert_eq!(parsed_deploy.action.as_deref(), Some("deploy_create"));
    assert_eq!(
        deploy_binding.app_id,
        parsed_deploy.app_id.unwrap_or_default()
    );
    assert_eq!(
        deploy_binding.branch,
        parsed_deploy.branch.unwrap_or_default()
    );
    assert_eq!(
        deploy_binding.commit_sha,
        parsed_deploy.commit_sha.unwrap_or_default()
    );
    assert_eq!(deploy_binding.context, parsed_deploy.context);

    mint_token(deploy_binding.clone(), 60).unwrap();
    let mut actual_deploy = deploy_binding;
    actual_deploy.tool_call_id = "actual-session:toolu_deploy".into();
    actual_deploy.synthesized_by_helper = false;
    assert!(verify_or_claim_token(actual_deploy).valid);
}

#[test]
fn consent_rejects_symlink_and_world_readable_private_files_on_unix() {
    #[cfg(unix)]
    {
        use axhub_helpers::consent::key::{read_private_file, write_private_file_no_follow};
        use std::os::unix::fs::{symlink, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        let link = tmp.path().join("link");
        fs::write(&target, b"secret").unwrap();
        symlink(&target, &link).unwrap();
        assert!(write_private_file_no_follow(&link, b"new").is_err());
        let public = tmp.path().join("public");
        fs::write(&public, b"secret").unwrap();
        fs::set_permissions(&public, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_private_file(&public).is_err());
    }
}

#[test]
fn keychain_parses_go_keyring_envelope() {
    let body =
        json!({"access_token":"axhub_pat_abcdefghijklmnop","token_type":"Bearer"}).to_string();
    let encoded = base64::engine::general_purpose::STANDARD.encode(body);
    assert_eq!(
        parse_keyring_value(&format!("go-keyring-base64:{encoded}")),
        Some("axhub_pat_abcdefghijklmnop".into())
    );
    assert_eq!(parse_keyring_value("not-base64"), None);
}

fn keychain_success_runner(cmd: &[&str], timeout_ms: u64) -> CommandOutput {
    assert_eq!(timeout_ms, 5000);
    assert!(!cmd.is_empty());
    let body =
        json!({"access_token":"axhub_pat_abcdefghijklmnop","token_type":"Bearer"}).to_string();
    CommandOutput {
        exit_code: 0,
        stdout: base64::engine::general_purpose::STANDARD.encode(body),
        stderr: String::new(),
    }
}

fn keychain_missing_runner(_cmd: &[&str], _timeout_ms: u64) -> CommandOutput {
    CommandOutput {
        exit_code: 1,
        stdout: String::new(),
        stderr: "not found".into(),
    }
}

fn keychain_bad_value_runner(_cmd: &[&str], _timeout_ms: u64) -> CommandOutput {
    CommandOutput {
        exit_code: 0,
        stdout: "not-base64".into(),
        stderr: String::new(),
    }
}

#[test]
fn keychain_runner_maps_platform_success_missing_parse_error_and_unsupported() {
    assert_eq!(parse_keyring_value(""), None);
    let helper_bin = env!("CARGO_BIN_EXE_axhub-helpers");
    let default_ok = axhub_helpers::keychain::default_runner(&[helper_bin, "version"], 1000);
    assert_eq!(default_ok.exit_code, 0);
    assert!(default_ok.stdout.contains("axhub-helpers "));
    let default_err = axhub_helpers::keychain::default_runner(&["/definitely-not-a-command"], 1000);
    assert_ne!(default_err.exit_code, 0);
    assert!(!default_err.stderr.is_empty());

    let mac = read_keychain_token_with_runner("macos", keychain_success_runner);
    assert_eq!(mac.source.as_deref(), Some("macos-keychain"));
    assert_eq!(mac.token.as_deref(), Some("axhub_pat_abcdefghijklmnop"));

    let linux_missing = read_keychain_token_with_runner("linux", keychain_missing_runner);
    assert!(linux_missing.error.unwrap().contains("secret-service"));

    let parse_error = read_keychain_token_with_runner("macos", keychain_bad_value_runner);
    assert!(parse_error.error.unwrap().contains("파싱할 수 없어요"));

    let unsupported = read_keychain_token_with_runner("plan9", keychain_success_runner);
    assert!(unsupported.error.unwrap().contains("지원하지 않는 플랫폼"));
}

fn windows_ok_runner(cmd: &[&str], timeout_ms: u64) -> WindowsSpawnResult {
    assert_eq!(timeout_ms, PS_TIMEOUT_MS);
    assert_eq!(cmd[0], "powershell.exe");
    let envelope =
        json!({"access_token":"axhub_pat_windows123456","token_type":"Bearer"}).to_string();
    let keyring_blob = format!(
        "go-keyring-base64:{}",
        base64::engine::general_purpose::STANDARD.encode(envelope)
    );
    WindowsSpawnResult {
        exit_code: 0,
        signal_code: None,
        stdout: format!(
            "AXHUB_OK:{}",
            base64::engine::general_purpose::STANDARD.encode(keyring_blob)
        ),
        stderr: String::new(),
    }
}

fn windows_not_found_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: 0,
        signal_code: None,
        stdout: "ERR:NOT_FOUND\n".into(),
        stderr: String::new(),
    }
}

fn windows_load_fail_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: 1,
        signal_code: None,
        stdout: "ERR:LOAD_FAIL\n".into(),
        stderr: String::new(),
    }
}

fn windows_execution_policy_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: 1,
        signal_code: None,
        stdout: String::new(),
        stderr: "AuthorizationManager check failed".into(),
    }
}

fn windows_generic_fail_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: 1,
        signal_code: None,
        stdout: "unexpected".into(),
        stderr: "powershell missing".into(),
    }
}

fn windows_invalid_base64_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: 0,
        signal_code: None,
        stdout: "AXHUB_OK:not-base64".into(),
        stderr: String::new(),
    }
}

fn windows_no_token_blob_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: 0,
        signal_code: None,
        stdout: format!(
            "AXHUB_OK:{}",
            base64::engine::general_purpose::STANDARD.encode("not-a-keyring-token")
        ),
        stderr: String::new(),
    }
}

fn windows_edr_runner(_cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    WindowsSpawnResult {
        exit_code: -1,
        signal_code: None,
        stdout: String::new(),
        stderr: "blocked".into(),
    }
}

#[test]
fn windows_keychain_runner_covers_success_and_failure_guidance() {
    let helper_bin = env!("CARGO_BIN_EXE_axhub-helpers");
    let default_ok =
        axhub_helpers::keychain_windows::default_windows_runner(&[helper_bin, "version"], 1000);
    assert_eq!(default_ok.exit_code, 0);
    assert!(default_ok.stdout.contains("axhub-helpers "));
    let default_err = axhub_helpers::keychain_windows::default_windows_runner(
        &["/definitely-not-a-command"],
        1000,
    );
    assert_ne!(default_err.exit_code, 0);
    assert!(!default_err.stderr.is_empty());

    let ok = read_windows_keychain_with_runner(windows_ok_runner);
    assert_eq!(ok.token.as_deref(), Some("axhub_pat_windows123456"));
    assert_eq!(ok.source.as_deref(), Some("windows-credential-manager"));

    assert!(read_windows_keychain_with_runner(windows_not_found_runner)
        .error
        .unwrap()
        .contains("Credential Manager"));
    assert!(read_windows_keychain_with_runner(windows_load_fail_runner)
        .error
        .unwrap()
        .contains("Add-Type"));
    assert!(
        read_windows_keychain_with_runner(windows_execution_policy_runner)
            .error
            .unwrap()
            .contains("ExecutionPolicy")
    );
    assert!(
        read_windows_keychain_with_runner(windows_generic_fail_runner)
            .error
            .unwrap()
            .contains("PowerShell 실행 자체")
    );
    assert!(
        read_windows_keychain_with_runner(windows_invalid_base64_runner)
            .error
            .unwrap()
            .contains("base64 decode")
    );
    assert!(
        read_windows_keychain_with_runner(windows_no_token_blob_runner)
            .error
            .unwrap()
            .contains("Credential Manager")
    );
    assert!(read_windows_keychain_with_runner(windows_edr_runner)
        .error
        .unwrap()
        .contains("보안 솔루션"));

    assert_eq!(
        decode_windows_blob(
            &base64::engine::general_purpose::STANDARD.encode("plain-token-envelope")
        ),
        Some("plain-token-envelope".into())
    );
    let utf16_bytes: Vec<u8> = "plain-token-envelope"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(
        decode_windows_blob(&base64::engine::general_purpose::STANDARD.encode(utf16_bytes)),
        Some("plain-token-envelope".into())
    );
    assert!(decode_windows_blob("not-base64").is_none());
    assert!(is_edr_signal(&WindowsSpawnResult {
        exit_code: 0,
        signal_code: Some("9".into()),
        stdout: String::new(),
        stderr: String::new(),
    }));
    assert!(is_edr_signal(&WindowsSpawnResult {
        exit_code: 0xC0000409u32 as i32,
        signal_code: None,
        stdout: String::new(),
        stderr: String::new(),
    }));
}

#[test]
fn telemetry_is_opt_in_private_jsonl_and_error_swallowing() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&[
        "AXHUB_TELEMETRY",
        "XDG_STATE_HOME",
        "CLAUDE_SESSION_ID",
        "CLAUDECODE_SESSION_ID",
        "HOME",
        "PATH",
    ]);
    std::env::set_var("HOME", guard.path("home"));
    std::env::remove_var("XDG_STATE_HOME");
    assert!(state_dir().ends_with("home/.local/state/axhub-plugin"));

    let bin_dir = guard.path("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let axhub =
        std::path::Path::new(&bin_dir).join(if cfg!(windows) { "axhub.cmd" } else { "axhub" });
    let write_axhub = |version: &str| {
        let tmp_axhub = axhub.with_extension("tmp");
        let script = if cfg!(windows) {
            format!("@echo off\r\necho axhub {version}\r\n")
        } else {
            format!("#!/bin/sh\necho 'axhub {version}'\n")
        };
        fs::write(&tmp_axhub, script).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&tmp_axhub, fs::Permissions::from_mode(0o755)).unwrap();
        #[cfg(windows)]
        let _ = fs::remove_file(&axhub);
        fs::rename(&tmp_axhub, &axhub).unwrap();
    };
    let helper_version = env!("CARGO_PKG_VERSION");
    write_axhub("0.0.1");
    let mut path_entries = vec![std::path::PathBuf::from(&bin_dir)];
    if let Some(old_path) = std::env::var_os("PATH") {
        path_entries.extend(std::env::split_paths(&old_path));
    }
    std::env::set_var("PATH", std::env::join_paths(path_entries).unwrap());
    reset_cli_version_cache();
    assert_eq!(resolve_cli_version(), "0.0.1");
    write_axhub(helper_version);
    assert_eq!(resolve_cli_version(), helper_version);
    write_axhub("9.9.9");
    assert_eq!(resolve_cli_version(), helper_version);
    reset_cli_version_cache();

    std::env::set_var("XDG_STATE_HOME", guard.path("state"));
    std::env::set_var("CLAUDE_SESSION_ID", "test_session_abc123");
    std::env::remove_var("AXHUB_TELEMETRY");
    reset_cli_version_cache();

    let mut fields = Map::new();
    fields.insert("event".into(), Value::String("test_event".into()));
    emit_meta_envelope(fields.clone()).unwrap();
    assert!(!std::path::Path::new(&guard.path("state/axhub-plugin/usage.jsonl")).exists());

    std::env::set_var("AXHUB_TELEMETRY", "1");
    emit_meta_envelope(fields).unwrap();
    let file = guard.path("state/axhub-plugin/usage.jsonl");
    let line = fs::read_to_string(&file).unwrap();
    let envelope: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(envelope["event"], "test_event");
    assert_eq!(envelope["session_id"], "test_session_abc123");
    assert!(envelope["ts"].as_str().unwrap().ends_with('Z'));
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );

    std::env::set_var("XDG_STATE_HOME", "/dev/null/cannot-write-here");
    let mut fields = Map::new();
    fields.insert("event".into(), Value::String("swallowed".into()));
    emit_meta_envelope(fields).unwrap();
}

#[test]
fn spawn_sync_covers_empty_command_and_successful_child_output() {
    let empty = axhub_helpers::spawn::spawn_sync(&[]).unwrap_err();
    assert!(empty.to_string().contains("command is empty"));

    let helper_bin = env!("CARGO_BIN_EXE_axhub-helpers");
    let helper = axhub_helpers::spawn::spawn_sync(&[helper_bin, "version"]).unwrap();
    assert_eq!(helper.exit_code, Some(0));
    assert!(helper.stdout.contains("axhub-helpers "));
}

#[test]
fn preauth_deny_hint_paydrop_baseline_locked() {
    let hint = format_preauth_deny_hint(Some("deploy_create"), Some("paydrop"));
    assert!(hint.contains("paydrop 배포해"), "got: {hint}");
    assert!(hint.contains("사전 승인"), "got: {hint}");
    assert!(hint.contains("승인 카드"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_uses_dynamic_app_for_deploy() {
    let hint = format_preauth_deny_hint(Some("deploy_create"), Some("shopmall"));
    assert!(hint.contains("'shopmall 배포해'"), "got: {hint}");
    assert!(
        !hint.contains("paydrop"),
        "stale paydrop in dynamic hint: {hint}"
    );
}

#[test]
fn preauth_deny_hint_apps_delete_uses_korean_imperative() {
    let hint = format_preauth_deny_hint(Some("apps_delete"), Some("oldapp"));
    assert!(hint.contains("'oldapp 앱 지워'"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_env_set_routes_to_env_skill() {
    let hint = format_preauth_deny_hint(Some("env_set"), Some("paydrop"));
    assert!(hint.contains("'paydrop 환경변수 추가해'"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_env_delete_routes_to_env_skill() {
    let hint = format_preauth_deny_hint(Some("env_delete"), Some("paydrop"));
    assert!(hint.contains("'paydrop 환경변수 삭제해'"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_tables_routes_to_tables_skill_not_rephrase() {
    let hint = format_preauth_deny_hint(Some("tables_create"), Some("ultraqa-app"));
    assert!(hint.contains("Skill(axhub:tables)"), "got: {hint}");
    assert!(hint.contains("consent-mint"), "got: {hint}");
    assert!(hint.contains("preview"), "got: {hint}");
    assert!(
        !hint.contains("라고 말해서 승인 카드를 받으세요"),
        "tables hint must not ask the user to rephrase: {hint}"
    );
}

#[test]
fn preauth_deny_hint_auth_login_omits_app_token() {
    let hint = format_preauth_deny_hint(Some("auth_login"), None);
    assert!(hint.contains("'로그인해'"), "got: {hint}");
    assert!(
        !hint.contains("앱이름"),
        "auth hint must not pad app placeholder: {hint}"
    );
}

#[test]
fn preauth_deny_hint_profile_use_omits_app_token() {
    let hint = format_preauth_deny_hint(Some("profile_use"), None);
    assert!(hint.contains("'profile 바꿔'"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_github_connect_includes_app() {
    let hint = format_preauth_deny_hint(Some("github_connect"), Some("paydrop"));
    assert!(hint.contains("'paydrop github 연결해'"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_unknown_action_falls_back_to_deploy_phrase() {
    let hint = format_preauth_deny_hint(None, None);
    assert!(hint.contains("'앱이름 배포해'"), "got: {hint}");
}

#[test]
fn preauth_deny_hint_empty_app_uses_placeholder() {
    let hint = format_preauth_deny_hint(Some("deploy_create"), Some(""));
    assert!(hint.contains("'앱이름 배포해'"), "got: {hint}");
}

#[test]
fn bootstrap_backend_contract_fixtures_lock_defaults_and_stops() {
    let success: Value = serde_json::from_str(include_str!(
        "fixtures/bootstrap/apps_create.success.v1.json"
    ))
    .unwrap();
    match interpret_apps_create_result(0, &success) {
        AppsCreateDecision::Registered(app) => {
            assert_eq!(app.app_id, "app_01HUBPAYDROP");
            assert_eq!(app.app_slug, "paydrop");
            assert_eq!(app.subdomain, "paydrop");
            assert_eq!(app.domain_id, "dom_01HUBDEFAULT");
        }
        other => panic!("expected registered app, got {other:?}"),
    }

    let alias_payload = json!({
        "id": 42,
        "slug": "legacy-paydrop",
        "subdomain": "legacy-paydrop",
        "domain_id": 1
    });
    match interpret_apps_create_result(0, &alias_payload) {
        AppsCreateDecision::Registered(app) => {
            assert_eq!(app.app_id, "42");
            assert_eq!(app.domain_id, "1");
        }
        other => panic!("expected registered numeric app/domain ids, got {other:?}"),
    }

    let missing_defaults: Value = serde_json::from_str(include_str!(
        "fixtures/bootstrap/apps_create.missing_defaults.json"
    ))
    .unwrap();
    assert!(matches!(
        interpret_apps_create_result(0, &missing_defaults),
        AppsCreateDecision::Stop {
            state: BootstrapState::BackendContractMissingDefaults,
            ..
        }
    ));

    let collision: Value = serde_json::from_str(include_str!(
        "fixtures/bootstrap/apps_create.422.subdomain_collision.json"
    ))
    .unwrap();
    match interpret_apps_create_result(422, &collision) {
        AppsCreateDecision::Stop {
            state: BootstrapState::SubdomainCollision,
            suggested_subdomain,
            ..
        } => assert_eq!(suggested_subdomain.as_deref(), Some("paydrop-2")),
        other => panic!("expected subdomain collision stop, got {other:?}"),
    }

    let legacy_collision = json!({
        "code": "subdomain_collision",
        "suggested_subdomain": "paydrop-3"
    });
    match interpret_apps_create_result(422, &legacy_collision) {
        AppsCreateDecision::Stop {
            state: BootstrapState::SubdomainCollision,
            suggested_subdomain,
            ..
        } => assert_eq!(suggested_subdomain.as_deref(), Some("paydrop-3")),
        other => panic!("expected legacy subdomain collision stop, got {other:?}"),
    }

    let server_error: Value =
        serde_json::from_str(include_str!("fixtures/bootstrap/apps_create.5xx.json")).unwrap();
    assert!(matches!(
        interpret_apps_create_result(500, &server_error),
        AppsCreateDecision::Stop {
            state: BootstrapState::IdempotencyUnavailable,
            ..
        }
    ));
}

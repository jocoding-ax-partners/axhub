use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

use axhub_helpers::axhub_cli::CliOutput;
use axhub_helpers::bootstrap::{interpret_apps_create_result, AppsCreateDecision, BootstrapState};
use axhub_helpers::catalog::classify;
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
fn runtime_paths_reject_symlink_and_world_readable_private_files_on_unix() {
    #[cfg(unix)]
    {
        use axhub_helpers::runtime_paths::{read_private_file, write_private_file_no_follow};
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
fn runtime_paths_private_file_roundtrip_and_dir_mode_on_unix() {
    #[cfg(unix)]
    {
        use axhub_helpers::runtime_paths::{
            read_private_file, set_private_dir_mode, write_private_file_no_follow,
        };
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();

        // write happy path: file is created 0600 and round-trips through read.
        let secret = tmp.path().join("token");
        write_private_file_no_follow(&secret, b"axhub-secret").unwrap();
        assert_eq!(read_private_file(&secret).unwrap(), b"axhub-secret");
        let fmode = fs::symlink_metadata(&secret).unwrap().permissions().mode() & 0o777;
        assert_eq!(fmode, 0o600);

        // set_private_dir_mode strips group/world bits down to 0700.
        let dir = tmp.path().join("state");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        set_private_dir_mode(&dir).unwrap();
        let dmode = fs::symlink_metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(dmode & 0o077, 0);
        assert_eq!(dmode, 0o700);
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

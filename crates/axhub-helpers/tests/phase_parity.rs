use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

use axhub_helpers::catalog::classify;
use axhub_helpers::consent::{
    mint_token, parse_axhub_command, verify_or_claim_token, verify_token, ConsentBinding,
};
use axhub_helpers::keychain::{
    parse_keyring_value, read_keychain_token_with_runner, CommandOutput,
};
use axhub_helpers::keychain_windows::{
    decode_windows_blob, is_edr_signal, read_windows_keychain_with_runner, WindowsSpawnResult,
    PS_TIMEOUT_MS,
};
use axhub_helpers::list_deployments::{
    pinned_hub_api_url, proxy_override_enabled, resolve_endpoint, resolve_token,
    run_list_deployments_with_fetch, spki_hash_from_cert_der, verify_hub_api_tls_pin, HttpResponse,
    ListDeploymentsArgs, TlsPinError, DEFAULT_ENDPOINT, EXIT_LIST_AUTH, EXIT_LIST_NOT_FOUND,
    EXIT_LIST_OK, EXIT_LIST_TRANSPORT,
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
use axhub_helpers::telemetry::{
    emit_meta_envelope, reset_cli_version_cache, resolve_cli_version, state_dir,
};
use base64::Engine;
use serde_json::{json, Map, Value};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn ok_tls_pin(_: &str) -> Result<(), axhub_helpers::list_deployments::TlsPinError> {
    Ok(())
}

fn env_lock() -> &'static Mutex<()> {
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

fn base_binding() -> ConsentBinding {
    ConsentBinding {
        tool_call_id: "sess-abc:tc-1".into(),
        action: "deploy_create".into(),
        app_id: "paydrop".into(),
        profile: "prod".into(),
        branch: "main".into(),
        commit_sha: "a3f9c1b".into(),
        context: HashMap::new(),
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
        redact(
            "\x1b[32mOK\x1b[0m Bearer abcdef1234567890abcdef AXHUB_TOKEN=xyz1234567890abcdef1234"
        ),
        "OK Bearer *** AXHUB_TOKEN=***"
    );
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
    assert!(classify(
        66,
        r#"{"error":{"code":"update.cosign_verification_failed"}}"#
    )
    .action
    .contains("IT 보안 담당자"));
    assert!(classify(99, "not-json{{").cause.contains("알 수 없는 에러"));
    assert!(
        classify(64, r#"{"error":{"code":"env.prod_force_required"}}"#)
            .action
            .contains("값은 절대")
    );
    assert!(
        classify(67, r#"{"error":{"code":"github.install_not_found"}}"#)
            .button
            .is_some_and(|button| button.contains("설치 URL"))
    );
    assert!(classify(
        66,
        r#"{"error":{"code":"profile.endpoint_not_in_allowlist"}}"#
    )
    .emotion
    .contains("허용 목록"));
    assert!(
        classify(65, r#"{"error":{"code":"apis.call_consent_required"}}"#)
            .cause
            .contains("서버 상태")
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
        ["axhub", "--version"] => SpawnResult { exit_code: EXIT_OK, stdout: "axhub 0.1.23".into(), stderr: String::new() },
        ["axhub", "auth", "status", "--json"] => SpawnResult { exit_code: EXIT_OK, stdout: r#"{"user_email":"u@example.com","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy"]}"#.into(), stderr: String::new() },
        _ => SpawnResult { exit_code: 1, stdout: String::new(), stderr: String::new() },
    }
    });
    assert_eq!(run.exit_code, EXIT_OK);
    assert_eq!(run.output.user_email.as_deref(), Some("u@example.com"));
    let too_new = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.11.0".into(),
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
fn preflight_admits_axhub_cli_0_10_line_and_rejects_0_11_exclusive_max() {
    for version in ["0.1.0", "0.5.0", "0.7.5", "0.9.0", "0.10.2"] {
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

    let too_new = run_preflight_with_runner(|cmd| match cmd {
        ["axhub", "--version"] => SpawnResult {
            exit_code: EXIT_OK,
            stdout: "axhub 0.11.0".into(),
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
            stdout: "axhub 0.1.5".into(),
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
            stdout: "axhub 0.1.5".into(),
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
    assert_eq!(
        absent.output.auth_error_code.as_deref(),
        Some("cli_unavailable")
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
    assert_eq!(run.output.app_id, Some(42));
    assert_eq!(run.output.branch.as_deref(), Some("main"));
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
    assert_eq!(
        parse_apps_list(r#"[{"id":1,"slug":"paydrop"},{"id":"bad","slug":"skip"}]"#)
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
            id: 1,
            slug: "admin-paydrop".into(),
            name: None,
        }],
        "pay",
    );
    assert_eq!(contains_match[0].slug, "admin-paydrop");
    drop(guard);
}

#[test]
fn list_deployments_maps_auth_not_found_success_and_proxy_skip() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&[
        "AXHUB_TOKEN",
        "AXHUB_ENDPOINT",
        "AXHUB_ALLOW_PROXY",
        "XDG_CONFIG_HOME",
    ]);
    std::env::remove_var("AXHUB_TOKEN");
    std::env::set_var("XDG_CONFIG_HOME", guard.path("config"));
    let missing = run_list_deployments_with_fetch(
        ListDeploymentsArgs {
            app_id: "42".into(),
            limit: None,
        },
        |_url, _token| unreachable!(),
        None::<fn(&str) -> Result<(), axhub_helpers::list_deployments::TlsPinError>>,
    );
    assert_eq!(missing.exit_code, EXIT_LIST_AUTH);

    std::env::set_var("AXHUB_TOKEN", "axhub_pat_abcdefghijklmnop");
    std::env::set_var("AXHUB_ENDPOINT", "https://example.test");
    let bad = run_list_deployments_with_fetch(
        ListDeploymentsArgs {
            app_id: "paydrop".into(),
            limit: None,
        },
        |_url, _token| unreachable!(),
        None::<fn(&str) -> Result<(), axhub_helpers::list_deployments::TlsPinError>>,
    );
    assert_eq!(bad.exit_code, EXIT_LIST_NOT_FOUND);

    let ok = run_list_deployments_with_fetch(
        ListDeploymentsArgs {
            app_id: "42".into(),
            limit: Some(1),
        },
        |url, token| {
            assert_eq!(
                url,
                "https://example.test/api/v1/apps/42/deployments?per_page=1"
            );
            assert_eq!(token, "axhub_pat_abcdefghijklmnop");
            Ok(HttpResponse { status: 200, body: r#"{"data":{"deployments":[{"id":7,"app_id":42,"status":3,"commit_sha":"abc","commit_message":"ship","branch":"main","created_at":"2026-04-29T00:00:00Z"}]}}"#.into() })
        },
        Some(ok_tls_pin),
    );
    assert_eq!(ok.exit_code, EXIT_LIST_OK);
    assert_eq!(ok.deployments[0].status, "active");
}

#[test]
fn list_deployments_covers_token_endpoint_http_and_error_matrix() {
    let _lock = env_lock().lock().unwrap();
    let guard = EnvGuard::new(&[
        "AXHUB_TOKEN",
        "AXHUB_ENDPOINT",
        "AXHUB_ALLOW_PROXY",
        "XDG_CONFIG_HOME",
        "HOME",
    ]);
    std::env::remove_var("AXHUB_TOKEN");
    std::env::remove_var("AXHUB_ENDPOINT");
    std::env::remove_var("AXHUB_ALLOW_PROXY");
    std::env::set_var("XDG_CONFIG_HOME", guard.path("config"));
    fs::create_dir_all(guard.path("config/axhub-plugin")).unwrap();
    fs::write(
        guard.path("config/axhub-plugin/token"),
        " axhub_pat_filetoken123456 \n",
    )
    .unwrap();
    assert_eq!(
        resolve_token().as_deref(),
        Some("axhub_pat_filetoken123456")
    );
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::set_var("HOME", guard.path("home"));
    fs::create_dir_all(guard.path("home/.config/axhub-plugin")).unwrap();
    fs::write(
        guard.path("home/.config/axhub-plugin/token"),
        "axhub_pat_hometoken123456",
    )
    .unwrap();
    assert_eq!(
        resolve_token().as_deref(),
        Some("axhub_pat_hometoken123456")
    );
    std::env::set_var("XDG_CONFIG_HOME", guard.path("config"));
    assert_eq!(resolve_endpoint(), DEFAULT_ENDPOINT);
    assert!(!proxy_override_enabled());
    std::env::set_var("AXHUB_ALLOW_PROXY", "1");
    assert!(proxy_override_enabled());
    std::env::set_var("AXHUB_ENDPOINT", "https://example.test");
    assert_eq!(resolve_endpoint(), "https://example.test");

    assert_eq!(
        pinned_hub_api_url("not a url").unwrap_err().code,
        "security.endpoint_invalid"
    );
    assert_eq!(
        pinned_hub_api_url("http://hub-api.jocodingax.ai")
            .unwrap_err()
            .code,
        "security.tls_required"
    );
    assert!(pinned_hub_api_url("https://example.test")
        .unwrap()
        .is_none());
    assert!(pinned_hub_api_url(DEFAULT_ENDPOINT).unwrap().is_some());
    assert!(verify_hub_api_tls_pin("https://example.test").is_ok());
    assert!(verify_hub_api_tls_pin(DEFAULT_ENDPOINT).is_ok());
    #[cfg(coverage)]
    {
        std::env::remove_var("AXHUB_ALLOW_PROXY");
        assert_eq!(
            verify_hub_api_tls_pin(DEFAULT_ENDPOINT).unwrap_err().code,
            "security.tls_pin_failed"
        );
        std::env::set_var("AXHUB_ALLOW_PROXY", "1");
    }
    assert!(spki_hash_from_cert_der(b"not a der cert").is_err());
    assert!(HttpResponse {
        status: 204,
        body: String::new(),
    }
    .ok());
    assert!(!HttpResponse {
        status: 302,
        body: String::new(),
    }
    .ok());

    let args = ListDeploymentsArgs {
        app_id: "42".into(),
        limit: None,
    };
    for (status, expected_exit, expected_code) in [
        (401, EXIT_LIST_AUTH, "auth.token_invalid"),
        (403, EXIT_LIST_AUTH, "auth.token_invalid"),
        (404, EXIT_LIST_NOT_FOUND, "resource.app_not_found"),
        (500, EXIT_LIST_TRANSPORT, "http.500"),
    ] {
        let got = run_list_deployments_with_fetch(
            args.clone(),
            move |_url, _token| {
                Ok(HttpResponse {
                    status,
                    body: "{}".into(),
                })
            },
            Some(ok_tls_pin),
        );
        assert_eq!(got.exit_code, expected_exit);
        assert_eq!(got.error_code.as_deref(), Some(expected_code));
    }

    let invalid_json = run_list_deployments_with_fetch(
        args.clone(),
        |_url, _token| {
            Ok(HttpResponse {
                status: 200,
                body: "not json".into(),
            })
        },
        Some(ok_tls_pin),
    );
    assert_eq!(invalid_json.exit_code, EXIT_LIST_TRANSPORT);
    assert_eq!(
        invalid_json.error_code.as_deref(),
        Some("response.invalid_json")
    );

    let transport = run_list_deployments_with_fetch(
        args.clone(),
        |_url, _token| Err(anyhow::anyhow!("network down")),
        Some(ok_tls_pin),
    );
    assert_eq!(transport.exit_code, EXIT_LIST_TRANSPORT);
    assert_eq!(
        transport.error_code.as_deref(),
        Some("transport.network_error")
    );

    let tls = run_list_deployments_with_fetch(
        args.clone(),
        |_url, _token| unreachable!(),
        Some(|_endpoint: &str| Err(TlsPinError::new("pin mismatch", "security.tls_pin_failed"))),
    );
    assert_eq!(tls.exit_code, EXIT_LIST_TRANSPORT);
    assert_eq!(tls.error_code.as_deref(), Some("security.tls_pin_failed"));

    let all_statuses = run_list_deployments_with_fetch(
        args,
        |_url, _token| {
            Ok(HttpResponse {
                status: 200,
                body: r#"{"data":{"deployments":[
                {"id":1,"app_id":42,"status":0,"commit_sha":"a","created_at":"t"},
                {"id":2,"app_id":42,"status":1,"commit_sha":"b","commit_message":"m","branch":"dev","created_at":"t"},
                {"id":3,"app_id":42,"status":2,"commit_sha":"c","created_at":"t"},
                {"id":4,"app_id":42,"status":3,"commit_sha":"d","created_at":"t"},
                {"id":5,"app_id":42,"status":4,"commit_sha":"e","created_at":"t"},
                {"id":6,"app_id":42,"status":5,"commit_sha":"f","created_at":"t"},
                {"id":7,"app_id":42,"status":99,"commit_sha":"g","created_at":"t"}
            ]}}"#
                    .into(),
            })
        },
        Some(ok_tls_pin),
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

    #[cfg(coverage)]
    {
        std::env::set_var("AXHUB_TOKEN", "axhub_pat_coverage123456");
        std::env::set_var("AXHUB_ENDPOINT", "https://example.test");
        std::env::remove_var("AXHUB_ALLOW_PROXY");
        let deterministic =
            axhub_helpers::list_deployments::run_list_deployments(ListDeploymentsArgs {
                app_id: "42".into(),
                limit: Some(2),
            });
        assert_eq!(deterministic.exit_code, EXIT_LIST_OK);
        assert!(deterministic.deployments.is_empty());
    }
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

    let safe = parse_axhub_command("axhub deploy logs --app paydrop");
    assert!(!safe.is_destructive);
    assert!(safe.action.is_none());
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
            "axhub apps update paydrop --field name=Paydrop --json",
            "apps_update",
            Some("paydrop"),
            [("slug", "paydrop"), ("field", "name=Paydrop")].as_slice(),
        ),
        (
            "axhub apps delete paydrop --force --confirm=paydrop --json",
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
    let default_ok = axhub_helpers::keychain::default_runner(&["rustc", "--version"], 1000);
    assert_eq!(default_ok.exit_code, 0);
    assert!(default_ok.stdout.contains("rustc "));
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
    let default_ok =
        axhub_helpers::keychain_windows::default_windows_runner(&["rustc", "--version"], 1000);
    assert_eq!(default_ok.exit_code, 0);
    assert!(default_ok.stdout.contains("rustc "));
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
        let script = if cfg!(windows) {
            format!("@echo off\r\necho axhub {version}\r\n")
        } else {
            format!("#!/bin/sh\necho 'axhub {version}'\n")
        };
        fs::write(&axhub, script).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&axhub, fs::Permissions::from_mode(0o755)).unwrap();
    };
    write_axhub("1.2.3-beta.1");
    let mut path_entries = vec![std::path::PathBuf::from(&bin_dir)];
    if let Some(old_path) = std::env::var_os("PATH") {
        path_entries.extend(std::env::split_paths(&old_path));
    }
    std::env::set_var("PATH", std::env::join_paths(path_entries).unwrap());
    reset_cli_version_cache();
    assert_eq!(resolve_cli_version(), "1.2.3-beta.1");
    write_axhub("9.9.9");
    assert_eq!(resolve_cli_version(), "1.2.3-beta.1");
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

    let rustc = axhub_helpers::spawn::spawn_sync(&["rustc", "--version"]).unwrap();
    assert_eq!(rustc.exit_code, Some(0));
    assert!(rustc.stdout.contains("rustc "));
}

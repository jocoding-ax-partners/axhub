//! Phase 1 — `deploy_prep` composition tests.
//!
//! Spec: `.plan/deploy-time-reduction/phase-1-rest-dedup-statusline.md` §8.1.
//!
//! These tests exercise the pure composition function `compose_deploy_prep`
//! against hand-built `PreflightRun` and `ResolveRun` inputs so the merge logic
//! is verifiable without reproducing the full preflight/resolve internals.

use std::sync::Mutex;

use axhub_helpers::deploy_prep::{
    compose_deploy_prep, run_deploy_prep_with_runner, BootstrapPlan, DeployPrepResult,
};
use axhub_helpers::preflight::{PreflightOutput, PreflightRun, SpawnResult, EXIT_AUTH, EXIT_OK};
use axhub_helpers::resolve::{AppMatch, ResolveOutput, ResolveRun, EXIT_NOT_FOUND};

fn ok_preflight() -> PreflightRun {
    PreflightRun {
        output: PreflightOutput {
            cli_version: Some("0.12.1".into()),
            in_range: true,
            cli_too_old: false,
            cli_too_new: false,
            cli_present: true,
            auth_ok: true,
            auth_error_code: None,
            scopes: vec!["deploy:write".into()],
            profile: Some("prod".into()),
            endpoint: Some("https://api.axhub.dev".into()),
            user_email: Some("dev@jocodingax.ai".into()),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            expires_human: Some("never".into()),
            current_app: Some("paydrop".into()),
            current_env: Some("prod".into()),
            last_deploy_id: None,
            last_deploy_status: None,
            plugin_version: env!("CARGO_PKG_VERSION").into(),
        },
        exit_code: EXIT_OK,
    }
}

fn unauth_preflight() -> PreflightRun {
    let mut run = ok_preflight();
    run.output.auth_ok = false;
    run.output.auth_error_code = Some("auth.token_missing".into());
    run.exit_code = EXIT_AUTH;
    run
}

fn ok_resolve() -> ResolveRun {
    ResolveRun {
        output: ResolveOutput {
            profile: Some("prod".into()),
            endpoint: Some("https://api.axhub.dev".into()),
            app_id: Some(42),
            app_slug: Some("paydrop".into()),
            candidate_slug: Some("paydrop".into()),
            matched_apps: vec![AppMatch {
                id: 42,
                slug: "paydrop".into(),
            }],
            branch: Some("main".into()),
            commit_sha: Some("abc123".into()),
            commit_message: Some("hi".into()),
            git_repo: true,
            git_has_commit: true,
            git_init_needed: false,
            eta_sec: 60,
            error: None,
            github_repo_url: None,
        },
        exit_code: EXIT_OK,
    }
}

fn cold_resolve() -> ResolveRun {
    ResolveRun {
        output: ResolveOutput {
            profile: Some("prod".into()),
            endpoint: Some("https://api.axhub.dev".into()),
            app_id: None,
            app_slug: None,
            candidate_slug: Some("new-app".into()),
            matched_apps: vec![],
            branch: Some("main".into()),
            commit_sha: None,
            commit_message: None,
            git_repo: true,
            git_has_commit: false,
            git_init_needed: false,
            eta_sec: 60,
            error: None,
            github_repo_url: None,
        },
        exit_code: EXIT_OK,
    }
}

#[test]
fn warm_redeploy_merges_to_exit_zero_with_no_bootstrap_plan() {
    let result: DeployPrepResult = compose_deploy_prep(ok_preflight(), ok_resolve());
    assert_eq!(result.exit_code, EXIT_OK);
    assert!(result.preflight.auth_ok);
    assert_eq!(result.resolve.app_id, Some(42));
    assert!(result.bootstrap_plan.is_none());
}

#[test]
fn cold_first_deploy_emits_bootstrap_plan_and_exit_not_found() {
    let result = compose_deploy_prep(ok_preflight(), cold_resolve());
    assert_eq!(result.exit_code, EXIT_NOT_FOUND);
    let plan: BootstrapPlan = result.bootstrap_plan.expect("bootstrap_plan should be set");
    assert!(plan.is_first_deploy);
    assert!(plan.required_steps.contains(&"first_commit".to_string()));
    assert!(plan.required_steps.contains(&"apps_create".to_string()));
}

#[test]
fn run_deploy_prep_with_runner_dispatches_both_preflight_and_resolve() {
    // Orchestration smoke test: prove `run_deploy_prep_with_runner` actually
    // spawns the parallel std::thread::scope runner and routes calls through
    // the injected closure into both preflight and resolve. Recording call
    // shapes via a Mutex<Vec<...>> rules out a future regression that
    // silently drops one of the two subcommand paths.
    let calls: Mutex<Vec<Vec<String>>> = Mutex::new(Vec::new());
    let runner = |cmd: &[&str]| -> SpawnResult {
        calls
            .lock()
            .unwrap()
            .push(cmd.iter().map(|s| s.to_string()).collect());
        let last = cmd.last().copied().unwrap_or("");
        if cmd.contains(&"--version") {
            SpawnResult {
                exit_code: 0,
                stdout: "axhub 0.12.1\n".into(),
                stderr: String::new(),
            }
        } else if cmd.contains(&"auth") && cmd.contains(&"status") {
            SpawnResult {
                exit_code: 0,
                stdout: r#"{"user_email":"dev@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}"#.into(),
                stderr: String::new(),
            }
        } else if cmd.contains(&"apps") && cmd.contains(&"list") {
            SpawnResult {
                exit_code: 0,
                stdout: r#"[{"id":42,"slug":"paydrop"}]"#.into(),
                stderr: String::new(),
            }
        } else {
            // git context probes (rev-parse, log) — return empty success so
            // resolve's git-readiness logic falls through cleanly.
            let _ = last;
            SpawnResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    };

    let args = vec!["--intent".to_string(), "deploy".to_string()];
    let result = run_deploy_prep_with_runner(&args, runner);

    let recorded = calls.lock().unwrap();
    let saw_preflight_version = recorded.iter().any(|c| c.iter().any(|a| a == "--version"));
    let saw_auth_status = recorded
        .iter()
        .any(|c| c.contains(&"auth".to_string()) && c.contains(&"status".to_string()));
    let saw_apps_list = recorded
        .iter()
        .any(|c| c.contains(&"apps".to_string()) && c.contains(&"list".to_string()));
    assert!(
        saw_preflight_version,
        "preflight `--version` must be invoked"
    );
    assert!(saw_auth_status, "auth status must be invoked");
    assert!(saw_apps_list, "resolve `apps list` must be invoked");
    // Both threads ran — preflight produced auth_ok, resolve produced output.
    assert!(result.preflight.cli_present);
    assert!(result.preflight.auth_ok);
}

#[test]
fn unauth_preflight_short_circuits_exit_code_priority() {
    // Even with a successful resolve, an UNAUTH preflight must own the
    // surfaced exit code so the SKILL routes to login recovery.
    let result = compose_deploy_prep(unauth_preflight(), ok_resolve());
    assert_eq!(result.exit_code, EXIT_AUTH);
    assert!(!result.preflight.auth_ok);
    assert_eq!(result.resolve.app_id, Some(42));
}

// ── PR B1 (issue #81 M3) — selective refresh tests ────────────────────────────

use std::sync::Arc;

static B1_ENV_LOCK: Mutex<()> = Mutex::new(());

fn respond_stub(cmd: &[&str], apps_list: &str) -> SpawnResult {
    if cmd.contains(&"--version") {
        SpawnResult {
            exit_code: 0,
            stdout: "axhub 0.12.1\n".into(),
            stderr: String::new(),
        }
    } else if cmd.contains(&"auth") && cmd.contains(&"status") {
        SpawnResult {
            exit_code: 0,
            stdout: r#"{"user_email":"dev","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}"#.into(),
            stderr: String::new(),
        }
    } else if cmd.contains(&"apps") && cmd.contains(&"list") {
        SpawnResult {
            exit_code: 0,
            stdout: apps_list.to_string(),
            stderr: String::new(),
        }
    } else {
        SpawnResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

fn capturing_runner(
    calls: Arc<Mutex<Vec<Vec<String>>>>,
    apps_list: String,
) -> impl Fn(&[&str]) -> SpawnResult + Sync {
    move |cmd: &[&str]| {
        calls
            .lock()
            .unwrap()
            .push(cmd.iter().map(|s| s.to_string()).collect());
        respond_stub(cmd, &apps_list)
    }
}

fn deploy_args(refresh: bool) -> Vec<String> {
    let mut a = vec![
        "--intent".into(),
        "deploy".into(),
        "--user-utterance".into(),
        "paydrop".into(),
    ];
    if refresh {
        a.push("--refresh-in-flight".into());
    }
    a
}

fn with_temp_cache_home<F: FnOnce(&std::path::Path)>(f: F) {
    let _g = B1_ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::var_os("XDG_CACHE_HOME");
    std::env::set_var("XDG_CACHE_HOME", dir.path());
    f(dir.path());
    match prev {
        Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
        None => std::env::remove_var("XDG_CACHE_HOME"),
    }
}

#[test]
fn refresh_in_flight_with_cache_hit_skips_preflight_calls() {
    with_temp_cache_home(|_| {
        // Prime cache
        let calls1: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner1 = capturing_runner(
            Arc::clone(&calls1),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let _ = run_deploy_prep_with_runner(&deploy_args(false), runner1);

        // Refresh
        let calls2: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner2 = capturing_runner(
            Arc::clone(&calls2),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let _ = run_deploy_prep_with_runner(&deploy_args(true), runner2);

        let recorded = calls2.lock().unwrap();
        let preflight_called = recorded.iter().any(|c| c.iter().any(|a| a == "--version"));
        let resolve_called = recorded
            .iter()
            .any(|c| c.contains(&"apps".to_string()) && c.contains(&"list".to_string()));
        assert!(
            !preflight_called,
            "preflight (--version) must NOT be called on refresh cache hit"
        );
        assert!(
            resolve_called,
            "resolve (apps list) must be called on refresh"
        );
    });
}

#[test]
fn refresh_in_flight_cache_miss_falls_back_to_full_fetch() {
    with_temp_cache_home(|_| {
        // No prime — cache miss
        let calls: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner = capturing_runner(Arc::clone(&calls), r#"[{"id":42,"slug":"paydrop"}]"#.into());
        let _ = run_deploy_prep_with_runner(&deploy_args(true), runner);

        let recorded = calls.lock().unwrap();
        let preflight_called = recorded.iter().any(|c| c.iter().any(|a| a == "--version"));
        let resolve_called = recorded
            .iter()
            .any(|c| c.contains(&"apps".to_string()) && c.contains(&"list".to_string()));
        assert!(preflight_called, "cache miss must trigger full preflight");
        assert!(resolve_called, "cache miss must trigger resolve");
    });
}

#[test]
fn refresh_in_flight_repopulates_cache_for_next_call() {
    with_temp_cache_home(|cache_root| {
        // Prime
        let calls1: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner1 = capturing_runner(
            Arc::clone(&calls1),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let _ = run_deploy_prep_with_runner(&deploy_args(false), runner1);

        let cache_path = cache_root
            .join("axhub-plugin")
            .join("deploy-prep-cache.json");
        assert!(cache_path.exists(), "cache file should exist after prime");

        // Refresh with different resolve (empty apps list → app_id None)
        let calls2: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner2 = capturing_runner(Arc::clone(&calls2), "[]".into());
        let _ = run_deploy_prep_with_runner(&deploy_args(true), runner2);

        // Cache file must reflect fresh resolve (app_id=null)
        let json = std::fs::read_to_string(&cache_path).unwrap();
        let cache: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            cache["result"]["resolve"]["app_id"].is_null(),
            "cache must hold fresh resolve.app_id=null after refresh: {json}"
        );
    });
}

#[test]
fn normal_deploy_without_flag_does_full_fetch_and_writes_cache() {
    with_temp_cache_home(|cache_root| {
        let calls: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner = capturing_runner(Arc::clone(&calls), r#"[{"id":42,"slug":"paydrop"}]"#.into());
        let _ = run_deploy_prep_with_runner(&deploy_args(false), runner);

        let recorded = calls.lock().unwrap();
        let preflight_called = recorded.iter().any(|c| c.iter().any(|a| a == "--version"));
        let resolve_called = recorded
            .iter()
            .any(|c| c.contains(&"apps".to_string()) && c.contains(&"list".to_string()));
        assert!(preflight_called, "normal deploy must call preflight");
        assert!(resolve_called, "normal deploy must call resolve");

        let cache_path = cache_root
            .join("axhub-plugin")
            .join("deploy-prep-cache.json");
        assert!(cache_path.exists(), "normal deploy must write cache");
    });
}

#[test]
fn refresh_in_flight_rederives_bootstrap_plan_when_resolve_changes() {
    with_temp_cache_home(|_| {
        // Prime with app_id=Some → bootstrap_plan=None
        let calls1: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner1 = capturing_runner(
            Arc::clone(&calls1),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let primed = run_deploy_prep_with_runner(&deploy_args(false), runner1);
        assert!(
            primed.bootstrap_plan.is_none(),
            "primed cache should have no bootstrap_plan"
        );

        // Refresh with empty apps list → app_id=None → bootstrap_plan re-derived Some
        let calls2: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner2 = capturing_runner(Arc::clone(&calls2), "[]".into());
        let refreshed = run_deploy_prep_with_runner(&deploy_args(true), runner2);
        assert!(
            refreshed.bootstrap_plan.is_some(),
            "bootstrap_plan must be re-derived after refresh changes app_id"
        );
    });
}

#[test]
fn refresh_in_flight_rederives_github_connected_when_repo_url_changes() {
    with_temp_cache_home(|_| {
        // Prime without github_repo_url → github_connected=false
        let calls1: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner1 = capturing_runner(
            Arc::clone(&calls1),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let primed = run_deploy_prep_with_runner(&deploy_args(false), runner1);
        assert!(
            !primed.github_connected,
            "primed cache should have github_connected=false"
        );

        // Refresh with github_repo_url → github_connected=true (re-derived)
        let calls2: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner2 = capturing_runner(
            Arc::clone(&calls2),
            r#"[{"id":42,"slug":"paydrop","github_repo_url":"https://github.com/org/paydrop"}]"#
                .into(),
        );
        let refreshed = run_deploy_prep_with_runner(&deploy_args(true), runner2);
        assert!(
            refreshed.github_connected,
            "github_connected must be re-derived from fresh resolve.github_repo_url"
        );
    });
}

#[test]
fn refresh_in_flight_preserves_cached_at_timestamp() {
    // Sentinel-based verification — no timing dependency (issue #81 testing M2).
    // Prime cache then mutate cached_at to a stable sentinel string; selective
    // refresh must leave the sentinel intact while still updating resolve.
    with_temp_cache_home(|cache_root| {
        let calls1: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner1 = capturing_runner(
            Arc::clone(&calls1),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let _ = run_deploy_prep_with_runner(&deploy_args(false), runner1);

        let cache_path = cache_root
            .join("axhub-plugin")
            .join("deploy-prep-cache.json");

        // Overwrite cached_at with a sentinel that is (a) within the 300 s TTL so the
        // cache stays valid, (b) at a fixed past offset so we can detect any reset.
        // 60 s in the past keeps load_cache_with_timestamp returning Some(...) while
        // remaining distinguishable from chrono::Utc::now() at the refresh site.
        let sentinel_cached_at = (chrono::Utc::now() - chrono::Duration::seconds(60)).to_rfc3339();
        {
            let json = std::fs::read_to_string(&cache_path).unwrap();
            let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
            v["cached_at"] = serde_json::Value::String(sentinel_cached_at.clone());
            std::fs::write(&cache_path, serde_json::to_string(&v).unwrap()).unwrap();
        }

        let calls2: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let runner2 = capturing_runner(
            Arc::clone(&calls2),
            r#"[{"id":42,"slug":"paydrop"}]"#.into(),
        );
        let _ = run_deploy_prep_with_runner(&deploy_args(true), runner2);

        let post_cached_at = {
            let json = std::fs::read_to_string(&cache_path).unwrap();
            let v: serde_json::Value = serde_json::from_str(&json).unwrap();
            v["cached_at"].as_str().unwrap().to_string()
        };
        assert_eq!(
            sentinel_cached_at, post_cached_at,
            "cached_at sentinel must be preserved across selective refresh (TTL invariant)"
        );
    });
}

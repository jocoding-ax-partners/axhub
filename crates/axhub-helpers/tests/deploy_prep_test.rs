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

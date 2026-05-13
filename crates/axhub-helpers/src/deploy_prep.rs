//! Phase 1 — `deploy-prep` parallel preflight + resolve + bootstrap-plan helper.
//!
//! Spec: `.plan/deploy-time-reduction/phase-1-rest-dedup-statusline.md` §3.1.
//!
//! Replaces three SKILL.md sub-steps (preflight, resolve, repeat-resolve) with
//! one helper invocation that runs preflight and resolve in parallel via
//! `std::thread::scope` and emits a single JSON envelope.
//!
//! Exit code priority: preflight error wins over resolve error; first-deploy
//! requirement (no `app_id`) collapses to `EXIT_NOT_FOUND` (67) when neither
//! prior call surfaced an error.
//!
//! Phase 27 extension: in-flight deploy detection, github_connected flag,
//! recently_pushed_within_60s pre-computed routing flag. Kill switch:
//! `AXHUB_DEPLOY_IN_FLIGHT_CHECK=0`. Cache for `--refresh-in-flight`: TTL 300 s.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::list_deployments::InFlightDeploy;
use crate::preflight::{
    default_runner, run_preflight_with_runner, PreflightOutput, PreflightRun, SpawnResult, EXIT_OK,
};
use crate::resolve::{run_resolve_with_runner, ResolveOutput, ResolveRun, EXIT_NOT_FOUND};

/// Kill switch env var. Set to "0" to disable in-flight deploy detection.
pub const ENV_IN_FLIGHT_KILL_SWITCH: &str = "AXHUB_DEPLOY_IN_FLIGHT_CHECK";

/// Window (seconds) used when calling `find_app_in_flight_with_window`.
/// Intentionally separate from `recovery_scan::DEFAULT_STALE_THRESHOLD_SECS`
/// even though numerically equal — the two thresholds serve different purposes.
const IN_FLIGHT_WINDOW_SECS: u64 = 600;

/// Cache TTL for `--refresh-in-flight` mode (seconds).
const CACHE_TTL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapPlan {
    pub is_first_deploy: bool,
    pub required_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeployPrepResult {
    pub preflight: PreflightOutput,
    pub resolve: ResolveOutput,
    pub bootstrap_plan: Option<BootstrapPlan>,
    pub exit_code: i32,
    /// In-flight deploy for this app, if one exists within the detection window.
    /// Serialises as JSON `null` when absent (not missing key) via `#[serde(default)]`.
    /// JSON shape: `{"id": i64, "created_at": "<RFC3339>"}`.
    #[serde(default)]
    pub in_flight_deploy: Option<InFlightDeploy>,
    /// True when the resolved app has a linked GitHub repository.
    #[serde(default)]
    pub github_connected: bool,
}

fn derive_bootstrap_plan(resolve: &ResolveOutput) -> Option<BootstrapPlan> {
    if resolve.app_id.is_some() {
        return None;
    }
    let mut required_steps: Vec<String> = Vec::new();
    if resolve.git_init_needed {
        required_steps.push("git_init".to_string());
    }
    if !resolve.git_has_commit {
        required_steps.push("first_commit".to_string());
    }
    required_steps.push("template".to_string());
    required_steps.push("apps_create".to_string());
    Some(BootstrapPlan {
        is_first_deploy: true,
        required_steps,
    })
}

fn merge_exit_code(preflight_code: i32, resolve_code: i32, plan: Option<&BootstrapPlan>) -> i32 {
    if preflight_code != EXIT_OK {
        return preflight_code;
    }
    if resolve_code != EXIT_OK {
        return resolve_code;
    }
    if plan.is_some() {
        return EXIT_NOT_FOUND;
    }
    EXIT_OK
}

/// Returns true when in-flight deploy detection is active (kill switch not set).
pub fn in_flight_check_enabled() -> bool {
    std::env::var(ENV_IN_FLIGHT_KILL_SWITCH).as_deref() != Ok("0")
}

/// Apply in-flight deploy information to an assembled result.
///
/// Respects the kill switch: if `AXHUB_DEPLOY_IN_FLIGHT_CHECK=0`, clears
/// `in_flight_deploy` regardless of the supplied value.
/// The SKILL layer computes timing comparisons (60 s window) deterministically
/// via shell `date` — no pre-computed flag is stored in the envelope.
pub fn apply_in_flight(result: &mut DeployPrepResult, in_flight: Option<InFlightDeploy>) {
    if !in_flight_check_enabled() {
        result.in_flight_deploy = None;
        return;
    }
    result.in_flight_deploy = in_flight;
}

/// Pure composition. Does NOT perform the in-flight HTTP check; callers that
/// want the full envelope should use `run_deploy_prep_with_runner` or call
/// `apply_in_flight` explicitly after fetching.
pub fn compose_deploy_prep(preflight: PreflightRun, resolve: ResolveRun) -> DeployPrepResult {
    let bootstrap_plan = derive_bootstrap_plan(&resolve.output);
    let exit_code = merge_exit_code(
        preflight.exit_code,
        resolve.exit_code,
        bootstrap_plan.as_ref(),
    );
    let github_connected = resolve.output.github_repo_url.is_some();
    DeployPrepResult {
        preflight: preflight.output,
        resolve: resolve.output,
        bootstrap_plan,
        exit_code,
        in_flight_deploy: None,
        github_connected,
    }
}

// ── File cache for --refresh-in-flight ────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct CacheFile {
    cached_at: String, // RFC3339
    result: DeployPrepResult,
}

fn cache_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cache")
        })
        .join("axhub-plugin");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("deploy-prep-cache.json"))
}

fn save_cache(result: &DeployPrepResult) {
    let Some(path) = cache_path() else { return };
    let cache = CacheFile {
        cached_at: chrono::Utc::now().to_rfc3339(),
        result: result.clone(),
    };
    let Ok(json) = serde_json::to_string(&cache) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

fn load_cache() -> Option<DeployPrepResult> {
    let path = cache_path()?;
    let json = std::fs::read_to_string(path).ok()?;
    let cache: CacheFile = serde_json::from_str(&json).ok()?;
    let cached_at = chrono::DateTime::parse_from_rfc3339(&cache.cached_at).ok()?;
    let age = chrono::Utc::now()
        .signed_duration_since(cached_at.with_timezone(&chrono::Utc))
        .num_seconds();
    if age < 0 || age as u64 > CACHE_TTL_SECS {
        return None;
    }
    Some(cache.result)
}

// ── Public entry points ────────────────────────────────────────────────────────

pub fn run_deploy_prep(args: &[String]) -> DeployPrepResult {
    run_deploy_prep_with_runner(args, default_runner)
}

pub fn run_deploy_prep_with_runner<F>(args: &[String], runner: F) -> DeployPrepResult
where
    F: Fn(&[&str]) -> SpawnResult + Sync,
{
    let refresh_in_flight = args.iter().any(|a| a == "--refresh-in-flight");

    let mut result = if refresh_in_flight {
        // Use cached preflight + resolve; only re-check in_flight below.
        if let Some(cached) = load_cache() {
            cached
        } else {
            // Cache miss or expired — fall back to full fresh fetch.
            run_preflight_and_resolve(args, &runner)
        }
    } else {
        let fresh = run_preflight_and_resolve(args, &runner);
        save_cache(&fresh);
        fresh
    };

    // In-flight detection (skipped by kill switch).
    if in_flight_check_enabled() {
        if let Some(app_id) = result.resolve.app_id {
            let now = chrono::Utc::now();
            #[cfg(not(coverage))]
            {
                use crate::list_deployments::find_app_in_flight_with_window;
                if let Ok(inflight) =
                    find_app_in_flight_with_window(app_id, now, IN_FLIGHT_WINDOW_SECS)
                {
                    apply_in_flight(&mut result, inflight);
                }
            }
            let _ = (app_id, now); // suppress unused warnings in coverage builds
        }
    }

    result
}

fn run_preflight_and_resolve<F>(args: &[String], runner: &F) -> DeployPrepResult
where
    F: Fn(&[&str]) -> SpawnResult + Sync,
{
    let (preflight_run, resolve_run) = std::thread::scope(|scope| {
        let preflight_handle = scope.spawn(move || run_preflight_with_runner(runner));
        let resolve_handle = scope.spawn(move || run_resolve_with_runner(args, runner));
        let preflight = preflight_handle.join().expect("preflight thread panicked");
        let resolve = resolve_handle.join().expect("resolve thread panicked");
        (preflight, resolve)
    });
    compose_deploy_prep(preflight_run, resolve_run)
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::list_deployments::InFlightDeploy;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn make_result(github_connected: bool) -> DeployPrepResult {
        DeployPrepResult {
            preflight: crate::preflight::PreflightOutput {
                cli_version: None,
                in_range: true,
                cli_too_old: false,
                cli_too_new: false,
                cli_present: true,
                auth_ok: true,
                auth_error_code: None,
                scopes: vec![],
                profile: None,
                endpoint: None,
                user_email: None,
                expires_at: None,
                expires_human: None,
                current_app: None,
                current_env: None,
                last_deploy_id: None,
                last_deploy_status: None,
                plugin_version: "0.0.0".into(),
            },
            resolve: crate::resolve::ResolveOutput {
                profile: None,
                endpoint: None,
                app_id: Some(1),
                app_slug: Some("test-app".into()),
                candidate_slug: None,
                matched_apps: vec![],
                branch: None,
                commit_sha: None,
                commit_message: None,
                git_repo: false,
                git_has_commit: false,
                git_init_needed: false,
                eta_sec: 60,
                error: None,
                github_repo_url: if github_connected {
                    Some("https://github.com/org/repo".into())
                } else {
                    None
                },
            },
            bootstrap_plan: None,
            exit_code: EXIT_OK,
            in_flight_deploy: None,
            github_connected,
        }
    }

    fn in_flight_deploy() -> InFlightDeploy {
        InFlightDeploy {
            id: 99,
            status: "building".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            commit_sha: "deadbeefcafe1234567890abcdef0123456789ab".into(),
            seconds_since_created: 30,
        }
    }

    /// `in_flight_deploy: None` → JSON `null`; `Some(...)` → nested object
    /// with `created_at` + `commit_sha` and no `seconds_since_created`.
    #[test]
    fn serializes_in_flight_deploy_field() {
        let mut result = make_result(false);

        // None → `"in_flight_deploy":null`
        let json_none = serde_json::to_string(&result).unwrap();
        assert!(
            json_none.contains("\"in_flight_deploy\":null"),
            "expected null: {json_none}"
        );

        // Some → nested object with `created_at` + `commit_sha`, no `seconds_since_created`
        result.in_flight_deploy = Some(in_flight_deploy());
        let json_some = serde_json::to_string(&result).unwrap();
        assert!(
            json_some.contains("\"in_flight_deploy\":{"),
            "expected nested object: {json_some}"
        );
        assert!(
            json_some.contains("\"created_at\":"),
            "field must be created_at: {json_some}"
        );
        assert!(
            json_some.contains("\"commit_sha\":\"deadbeefcafe"),
            "commit_sha must be present: {json_some}"
        );
        assert!(
            !json_some.contains("seconds_since_created"),
            "seconds_since_created must not appear in JSON: {json_some}"
        );
    }

    /// `commit_sha` 가 backend 응답에 없으면 default empty string 으로 deserialize 되어
    /// SKILL Step 1.6c (uncertain) 분기로 라우팅 가능.
    #[test]
    fn in_flight_deploy_deserializes_with_default_commit_sha() {
        let json = r#"{"id":42,"status":"building","created_at":"2024-01-01T00:00:00Z"}"#;
        let parsed: InFlightDeploy = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.id, 42);
        assert_eq!(parsed.commit_sha, "", "missing commit_sha → empty default");
    }

    /// Kill switch `AXHUB_DEPLOY_IN_FLIGHT_CHECK=0` must force in_flight_deploy to None.
    #[test]
    fn kill_switch_disables_in_flight_check() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var(ENV_IN_FLIGHT_KILL_SWITCH, "0");

        let mut result = make_result(true);
        apply_in_flight(&mut result, Some(in_flight_deploy()));

        std::env::remove_var(ENV_IN_FLIGHT_KILL_SWITCH);

        assert!(
            result.in_flight_deploy.is_none(),
            "kill switch must clear in_flight_deploy"
        );
    }

    /// save_cache → load_cache roundtrip preserves DeployPrepResult shape and
    /// exercises the atomic rename path (.tmp → final).
    #[test]
    fn save_cache_load_cache_roundtrip() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmpdir = std::env::temp_dir().join(format!(
            "axhub-test-cache-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmpdir).unwrap();
        let prev = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_CACHE_HOME", &tmpdir);

        let original = make_result(true);
        save_cache(&original);

        let loaded = load_cache().expect("expected cache hit after save_cache");
        assert_eq!(loaded.exit_code, original.exit_code);
        assert_eq!(loaded.github_connected, original.github_connected);

        // Verify no stray .tmp survives after rename.
        let cache_file = tmpdir.join("axhub-plugin").join("deploy-prep-cache.json");
        let tmp_file = cache_file.with_extension("json.tmp");
        assert!(cache_file.exists(), "final cache file must exist");
        assert!(!tmp_file.exists(), ".tmp must not survive rename");

        match prev {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    /// load_cache returns None when the cache file is missing or corrupt.
    #[test]
    fn load_cache_returns_none_on_missing_and_corrupt() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmpdir = std::env::temp_dir().join(format!(
            "axhub-test-cache-corrupt-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let prev = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_CACHE_HOME", &tmpdir);

        // Missing file → None
        assert!(
            load_cache().is_none(),
            "load_cache must return None for missing file"
        );

        // Corrupt file → None
        let cache_dir = tmpdir.join("axhub-plugin");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("deploy-prep-cache.json"), "not json").unwrap();
        assert!(
            load_cache().is_none(),
            "load_cache must return None for corrupt JSON"
        );

        match prev {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmpdir);
    }
}

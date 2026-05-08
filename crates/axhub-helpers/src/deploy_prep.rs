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

use serde::{Deserialize, Serialize};

use crate::preflight::{
    default_runner, run_preflight_with_runner, PreflightOutput, PreflightRun, SpawnResult, EXIT_OK,
};
use crate::resolve::{run_resolve_with_runner, ResolveOutput, ResolveRun, EXIT_NOT_FOUND};

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

pub fn compose_deploy_prep(preflight: PreflightRun, resolve: ResolveRun) -> DeployPrepResult {
    let bootstrap_plan = derive_bootstrap_plan(&resolve.output);
    let exit_code = merge_exit_code(
        preflight.exit_code,
        resolve.exit_code,
        bootstrap_plan.as_ref(),
    );
    DeployPrepResult {
        preflight: preflight.output,
        resolve: resolve.output,
        bootstrap_plan,
        exit_code,
    }
}

pub fn run_deploy_prep(args: &[String]) -> DeployPrepResult {
    run_deploy_prep_with_runner(args, default_runner)
}

pub fn run_deploy_prep_with_runner<F>(args: &[String], runner: F) -> DeployPrepResult
where
    F: Fn(&[&str]) -> SpawnResult + Sync,
{
    let runner_ref = &runner;
    let (preflight_run, resolve_run) = std::thread::scope(|scope| {
        let preflight_handle = scope.spawn(move || run_preflight_with_runner(runner_ref));
        let resolve_handle = scope.spawn(move || run_resolve_with_runner(args, runner_ref));
        let preflight = preflight_handle.join().expect("preflight thread panicked");
        let resolve = resolve_handle.join().expect("resolve thread panicked");
        (preflight, resolve)
    });
    compose_deploy_prep(preflight_run, resolve_run)
}

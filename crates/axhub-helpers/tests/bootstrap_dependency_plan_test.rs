use std::fs;

use axhub_helpers::bootstrap::{build_dependency_plan, PackageManager, PlanState};
use tempfile::tempdir;

fn write(path: &std::path::Path, name: &str, body: &str) {
    fs::write(path.join(name), body).expect("write fixture file");
}

#[test]
fn case1_npm_only_lockfile_install_required() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "package-lock.json", "{}\n");

    let plan = build_dependency_plan(cwd).expect("plan");

    assert_eq!(plan.detected_lockfile.as_deref(), Some("package-lock.json"));
    assert_eq!(plan.lockfile_count, 1);
    assert!(!plan.requires_pm_choice);
    assert_eq!(plan.manager_candidates, vec![PackageManager::Npm]);
    assert_eq!(plan.recommended_command.as_deref(), Some("npm install"));
    assert_eq!(plan.plan_state, PlanState::DependencyInstallRequired);
    assert!(plan.package_json_present);
    assert!(!plan.node_modules_present);
}

#[test]
fn case2_multiple_lockfiles_requires_pm_choice() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "package-lock.json", "{}\n");
    write(cwd, "pnpm-lock.yaml", "lockfileVersion: 6.0\n");

    let plan = build_dependency_plan(cwd).expect("plan");

    assert!(plan.requires_pm_choice);
    assert_eq!(plan.lockfile_count, 2);
    assert!(plan.detected_lockfile.is_none());
    assert!(plan.recommended_command.is_none());
    assert_eq!(plan.plan_state, PlanState::DependencyInstallRequired);
    assert!(plan
        .manager_candidates
        .iter()
        .any(|pm| matches!(pm, PackageManager::Npm)));
    assert!(plan
        .manager_candidates
        .iter()
        .any(|pm| matches!(pm, PackageManager::Pnpm)));
}

#[test]
fn case3_no_package_json_dependency_not_required() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();

    let plan = build_dependency_plan(cwd).expect("plan");

    assert_eq!(plan.lockfile_count, 0);
    assert!(!plan.requires_pm_choice);
    assert!(plan.detected_lockfile.is_none());
    assert!(plan.recommended_command.is_none());
    assert_eq!(plan.plan_state, PlanState::DependencyNotRequired);
    assert!(!plan.package_json_present);
    assert!(!plan.node_modules_present);
    assert_eq!(plan.manager_candidates, vec![PackageManager::Npm]);
}

#[test]
fn case4_node_modules_present_already_installed() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    fs::create_dir_all(cwd.join("node_modules")).expect("mkdir node_modules");

    let plan = build_dependency_plan(cwd).expect("plan");

    assert_eq!(plan.plan_state, PlanState::DependencyAlreadyInstalled);
    assert!(plan.package_json_present);
    assert!(plan.node_modules_present);
    assert!(plan.recommended_command.is_none());
    assert!(!plan.requires_pm_choice);
}

#[test]
fn case5_pnpm_only_lockfile_recommends_pnpm() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "pnpm-lock.yaml", "lockfileVersion: 6.0\n");

    let plan = build_dependency_plan(cwd).expect("plan");

    assert_eq!(plan.detected_lockfile.as_deref(), Some("pnpm-lock.yaml"));
    assert_eq!(plan.manager_candidates, vec![PackageManager::Pnpm]);
    assert_eq!(plan.recommended_command.as_deref(), Some("pnpm install"));
    assert_eq!(plan.plan_state, PlanState::DependencyInstallRequired);
}

#[test]
fn case6_yarn_only_lockfile_recommends_yarn() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "yarn.lock", "# yarn lockfile v1\n");

    let plan = build_dependency_plan(cwd).expect("plan");

    assert_eq!(plan.detected_lockfile.as_deref(), Some("yarn.lock"));
    assert_eq!(plan.manager_candidates, vec![PackageManager::Yarn]);
    assert_eq!(plan.recommended_command.as_deref(), Some("yarn install"));
}

#[test]
fn case7_bun_only_lockfile_recommends_bun() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "bun.lockb", "");

    let plan = build_dependency_plan(cwd).expect("plan");

    assert_eq!(plan.detected_lockfile.as_deref(), Some("bun.lockb"));
    assert_eq!(plan.manager_candidates, vec![PackageManager::Bun]);
    assert_eq!(plan.recommended_command.as_deref(), Some("bun install"));
}

#[test]
fn plan_state_as_str_covers_three_variants() {
    assert_eq!(
        PlanState::DependencyInstallRequired.as_str(),
        "dependency_install_required"
    );
    assert_eq!(
        PlanState::DependencyAlreadyInstalled.as_str(),
        "dependency_already_installed"
    );
    assert_eq!(
        PlanState::DependencyNotRequired.as_str(),
        "dependency_not_required"
    );
}

#[test]
fn package_manager_install_command_covers_four_variants() {
    assert_eq!(PackageManager::Npm.install_command(), "npm install");
    assert_eq!(PackageManager::Pnpm.install_command(), "pnpm install");
    assert_eq!(PackageManager::Yarn.install_command(), "yarn install");
    assert_eq!(PackageManager::Bun.install_command(), "bun install");
}

#[test]
fn cmd_dependency_plan_json_output_single_lockfile() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "package-lock.json", "{}\n");

    let exe = env!("CARGO_BIN_EXE_axhub-helpers");
    let output = std::process::Command::new(exe)
        .args(["bootstrap", "dependency-plan", "--json"])
        .current_dir(cwd)
        .output()
        .expect("spawn dependency-plan");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("\"plan_state\":\"dependency_install_required\""));
    assert!(stdout.contains("\"recommended_command\":\"npm install\""));
    assert!(stdout.contains("\"requires_pm_choice\":false"));
}

#[test]
fn cmd_dependency_plan_plain_output_single_lockfile() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "package-lock.json", "{}\n");

    let exe = env!("CARGO_BIN_EXE_axhub-helpers");
    let output = std::process::Command::new(exe)
        .args(["bootstrap", "dependency-plan"])
        .current_dir(cwd)
        .output()
        .expect("spawn dependency-plan plain");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("plan_state: dependency_install_required"));
    assert!(stdout.contains("recommended_command: npm install"));
}

#[test]
fn cmd_dependency_plan_multiple_lockfiles_exits_65() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();
    write(cwd, "package.json", "{}\n");
    write(cwd, "package-lock.json", "{}\n");
    write(cwd, "pnpm-lock.yaml", "lockfileVersion: 6.0\n");

    let exe = env!("CARGO_BIN_EXE_axhub-helpers");
    let output = std::process::Command::new(exe)
        .args(["bootstrap", "dependency-plan"])
        .current_dir(cwd)
        .output()
        .expect("spawn dependency-plan multi");

    assert_eq!(output.status.code(), Some(65));
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("multiple lockfiles detected"));
}

#[test]
fn cmd_dependency_plan_unknown_option_exits_64() {
    let dir = tempdir().expect("tempdir");
    let cwd = dir.path();

    let exe = env!("CARGO_BIN_EXE_axhub-helpers");
    let output = std::process::Command::new(exe)
        .args(["bootstrap", "dependency-plan", "--bogus-flag"])
        .current_dir(cwd)
        .output()
        .expect("spawn dependency-plan unknown");

    assert_eq!(output.status.code(), Some(64));
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(stderr.contains("unknown option"));
    assert!(stderr.contains("--bogus-flag"));
}

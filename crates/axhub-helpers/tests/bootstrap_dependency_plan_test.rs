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

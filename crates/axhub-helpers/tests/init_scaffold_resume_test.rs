use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn write(path: &std::path::Path, name: &str, body: &str) {
    fs::write(path.join(name), body).expect("write fixture");
}

fn expected_npm_executable() -> &'static str {
    #[cfg(windows)]
    {
        "npm.cmd"
    }
    #[cfg(not(windows))]
    {
        "npm"
    }
}

fn run_json(dir: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn axhub-helpers");
    let status = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let json: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout should be json ({err}):\n{stdout}");
    });
    (status, json)
}

#[test]
fn scaffold_detect_skips_when_package_json_is_absent() {
    let dir = tempdir().expect("tempdir");

    let (status, json) = run_json(dir.path(), &["scaffold-detect", "--json"]);

    assert_eq!(status, 0);
    assert_eq!(json["schema_version"], "scaffold-detect/v1");
    assert_eq!(json["package_json_present"], false);
    assert_eq!(json["can_install"], false);
    assert_eq!(json["can_start_dev"], false);
    assert_eq!(json["reason"], "package_json_missing");
}

#[test]
fn scaffold_detect_refuses_to_guess_without_lockfile() {
    let dir = tempdir().expect("tempdir");
    write(
        dir.path(),
        "package.json",
        r#"{"scripts":{"dev":"vite --host 127.0.0.1"}}"#,
    );

    let (status, json) = run_json(dir.path(), &["scaffold-detect", "--json"]);

    assert_eq!(status, 0);
    assert_eq!(json["package_json_present"], true);
    assert_eq!(json["lockfile_present"], false);
    assert_eq!(json["dev_script_present"], true);
    assert_eq!(json["can_install"], false);
    assert_eq!(json["can_start_dev"], false);
    assert_eq!(json["reason"], "lockfile_missing");
}

#[test]
fn scaffold_detect_reports_node_missing_without_executing_anything() {
    let dir = tempdir().expect("tempdir");
    write(dir.path(), "package.json", r#"{"scripts":{"dev":"vite"}}"#);
    write(dir.path(), "package-lock.json", "{}");

    let out = Command::new(bin())
        .args(["scaffold-detect", "--json"])
        .current_dir(dir.path())
        .env("AXHUB_SCAFFOLD_NODE_AVAILABLE", "0")
        .output()
        .expect("spawn scaffold-detect");
    assert_eq!(out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&out.stdout).expect("json stdout");

    assert_eq!(json["node_available"], false);
    assert_eq!(json["can_install"], false);
    assert_eq!(json["can_start_dev"], false);
    assert_eq!(json["reason"], "node_missing");
}

#[test]
fn scaffold_dev_dry_run_plans_install_with_ignore_scripts() {
    let dir = tempdir().expect("tempdir");
    write(
        dir.path(),
        "package.json",
        r#"{"scripts":{"dev":"vite --host 127.0.0.1"}}"#,
    );
    write(dir.path(), "package-lock.json", "{}");

    let out = Command::new(bin())
        .args(["scaffold-dev", "start", "--json"])
        .current_dir(dir.path())
        .env("AXHUB_SCAFFOLD_NODE_AVAILABLE", "1")
        .env("AXHUB_SCAFFOLD_DEV_DRY_RUN", "1")
        .output()
        .expect("spawn scaffold-dev start");
    assert_eq!(out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&out.stdout).expect("json stdout");

    assert_eq!(json["schema_version"], "scaffold-dev/v1");
    assert_eq!(json["action"], "planned");
    assert_eq!(json["install_command"][0], expected_npm_executable());
    assert!(
        json["install_command"]
            .as_array()
            .expect("install command array")
            .iter()
            .any(|v| v == "--ignore-scripts"),
        "install command must block lifecycle scripts: {json}"
    );
    assert_eq!(
        json["dev_command"],
        serde_json::json!([expected_npm_executable(), "run", "dev"])
    );
}

#[test]
fn scaffold_dev_start_skips_when_dev_script_is_missing() {
    let dir = tempdir().expect("tempdir");
    write(dir.path(), "package.json", "{}");
    write(dir.path(), "package-lock.json", "{}");

    let out = Command::new(bin())
        .args(["scaffold-dev", "start", "--json"])
        .current_dir(dir.path())
        .env("AXHUB_SCAFFOLD_NODE_AVAILABLE", "1")
        .env("AXHUB_SCAFFOLD_DEV_DRY_RUN", "1")
        .output()
        .expect("spawn scaffold-dev start");
    assert_eq!(out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&out.stdout).expect("json stdout");

    assert_eq!(json["action"], "skipped");
    assert_eq!(json["reason"], "dev_script_missing");
}

#[test]
fn scaffold_dev_start_does_not_report_ready_when_no_url_is_detected() {
    let dir = tempdir().expect("tempdir");
    write(
        dir.path(),
        "package.json",
        r#"{"scripts":{"dev":"node dev-no-url.js"} }"#,
    );
    write(dir.path(), "dev-no-url.js", "process.exit(0);\n");
    write(dir.path(), "package-lock.json", "{}");
    fs::create_dir_all(dir.path().join("node_modules")).expect("node_modules");

    let out = Command::new(bin())
        .args(["scaffold-dev", "start", "--json"])
        .current_dir(dir.path())
        .env("AXHUB_SCAFFOLD_NODE_AVAILABLE", "1")
        .output()
        .expect("spawn scaffold-dev start");
    assert_eq!(out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&out.stdout).expect("json stdout");

    assert_eq!(json["schema_version"], "scaffold-dev/v1");
    assert_eq!(json["ready"], false);
    assert!(
        json["reason"] == "dev_process_exited" || json["reason"] == "url_not_detected",
        "missing URL must not be reported as ready: {json}"
    );
}

#[test]
fn scaffold_dev_start_ignores_stale_urls_from_previous_log() {
    let dir = tempdir().expect("tempdir");
    write(
        dir.path(),
        "package.json",
        r#"{"scripts":{"dev":"node dev-stale-log.js"} }"#,
    );
    write(
        dir.path(),
        "dev-stale-log.js",
        "setTimeout(() => process.exit(0), 250);\n",
    );
    write(dir.path(), "package-lock.json", "{}");
    fs::create_dir_all(dir.path().join("node_modules")).expect("node_modules");
    let axhub_dir = dir.path().join(".axhub");
    fs::create_dir_all(&axhub_dir).expect("mkdir .axhub");
    fs::write(
        axhub_dir.join("scaffold-dev.log"),
        "stale prior run http://localhost:6553\n",
    )
    .expect("write stale log");

    let out = Command::new(bin())
        .args(["scaffold-dev", "start", "--json"])
        .current_dir(dir.path())
        .env("AXHUB_SCAFFOLD_NODE_AVAILABLE", "1")
        .output()
        .expect("spawn scaffold-dev start");
    assert_eq!(out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&out.stdout).expect("json stdout");

    assert_eq!(json["ready"], false);
    assert_ne!(json["url"], "http://localhost:6553");
}

#[test]
fn scaffold_dev_status_is_fail_soft_without_state() {
    let dir = tempdir().expect("tempdir");

    let (status, json) = run_json(dir.path(), &["scaffold-dev", "status", "--json"]);

    assert_eq!(status, 0);
    assert_eq!(json["schema_version"], "scaffold-dev/v1");
    assert_eq!(json["alive"], false);
    assert_eq!(json["reason"], "state_missing");
}

#[test]
fn scaffold_dev_status_rejects_stale_or_mismatched_state() {
    let dir = tempdir().expect("tempdir");
    let axhub_dir = dir.path().join(".axhub");
    fs::create_dir_all(&axhub_dir).expect("mkdir .axhub");
    let stale_state = serde_json::json!({
        "schema_version": "scaffold-dev/v1",
        "pid": std::process::id(),
        "url": "http://localhost:5173",
        "port": 5173,
        "command": [expected_npm_executable(), "run", "dev"],
        "log_path": axhub_dir.join("scaffold-dev.log").to_string_lossy(),
        "cwd": "/not/the/current/workdir",
        "started_at": "2000-01-01T00:00:00Z",
        "updated_at": "2000-01-01T00:00:00Z"
    });
    fs::write(
        axhub_dir.join("scaffold-dev.json"),
        serde_json::to_vec_pretty(&stale_state).expect("serialize stale state"),
    )
    .expect("write stale state");

    let (status, json) = run_json(dir.path(), &["scaffold-dev", "status", "--json"]);

    assert_eq!(status, 0);
    assert_eq!(json["alive"], false);
    assert_eq!(json["ready"], false);
    assert_eq!(json["reason"], "state_stale");
}

#[test]
fn init_resume_put_route_and_clear_are_repo_local() {
    let dir = tempdir().expect("tempdir");

    let (put_status, put_json) = run_json(
        dir.path(),
        &[
            "init-resume",
            "put",
            "--template",
            "nextjs",
            "--app-name",
            "결제 앱",
            "--slug",
            "pay-app",
            "--subdomain",
            "pay-app-abc",
            "--pending-device-flow",
            "true",
            "--json",
        ],
    );
    assert_eq!(put_status, 0);
    assert_eq!(put_json["state"]["schema_version"], "init-resume/v1");
    assert_eq!(put_json["state"]["template"], "nextjs");
    assert_eq!(put_json["state"]["app_name"], "결제 앱");
    assert!(dir.path().join(".axhub/init-resume.json").exists());

    let (_, route_json) = run_json(dir.path(), &["init-resume", "route", "--json"]);
    assert_eq!(route_json["route"], "resume_last");
    assert_eq!(route_json["args"]["template"], "nextjs");
    assert_eq!(route_json["args"]["name"], "결제 앱");
    assert_eq!(route_json["args"]["slug"], "pay-app");
    assert!(
        route_json["args"]["idempotency_key"]
            .as_str()
            .unwrap()
            .len()
            >= 32
    );

    let (_, clear_json) = run_json(dir.path(), &["init-resume", "clear", "--json"]);
    assert_eq!(clear_json["cleared"], true);
    assert!(!dir.path().join(".axhub/init-resume.json").exists());
}

#[test]
fn init_resume_route_uses_bootstrap_status_when_id_exists() {
    let dir = tempdir().expect("tempdir");
    let (_, put_json) = run_json(
        dir.path(),
        &[
            "init-resume",
            "put",
            "--template",
            "react",
            "--app-name",
            "관리 앱",
            "--slug",
            "admin-app",
            "--bootstrap-id",
            "00000000-0000-4000-8000-000000000001",
            "--repo-full-name",
            "acme/admin-app",
            "--json",
        ],
    );
    assert_eq!(
        put_json["state"]["bootstrap_id"],
        "00000000-0000-4000-8000-000000000001"
    );

    let (_, route_json) = run_json(dir.path(), &["init-resume", "route", "--json"]);

    assert_eq!(route_json["route"], "watch_status");
    assert_eq!(
        route_json["args"]["bootstrap_id"],
        "00000000-0000-4000-8000-000000000001"
    );
    assert_eq!(route_json["requires_status_authority"], true);
    assert_eq!(
        route_json["args"]["status_command"],
        serde_json::json!([
            "axhub",
            "apps",
            "bootstrap-status",
            "00000000-0000-4000-8000-000000000001",
            "--watch",
            "--watch-timeout",
            "9m",
            "--json"
        ])
    );
    assert_eq!(route_json["state"]["clone_done"], false);
}

#[test]
fn init_resume_route_marks_old_breadcrumb_state_stale() {
    let dir = tempdir().expect("tempdir");
    let axhub_dir = dir.path().join(".axhub");
    fs::create_dir_all(&axhub_dir).expect("mkdir .axhub");
    fs::write(
        axhub_dir.join("init-resume.json"),
        r#"{
  "schema_version": "init-resume/v1",
  "template": "nextjs",
  "app_name": "오래된 앱",
  "slug": "old-app",
  "subdomain": null,
  "idempotency_key": "00000000-0000-4000-8000-000000000002",
  "bootstrap_id": null,
  "repo_full_name": null,
  "clone_done": false,
  "pending_device_flow": true,
  "created_at": "2000-01-01T00:00:00Z",
  "updated_at": "2000-01-01T00:00:00Z"
}"#,
    )
    .expect("write old state");

    let (_, route_json) = run_json(dir.path(), &["init-resume", "route", "--json"]);

    assert_eq!(route_json["route"], "fresh");
    assert_eq!(route_json["reason"], "state_stale");
    assert_eq!(route_json["state_stale"], true);
    assert_eq!(route_json["requires_status_authority"], false);
}

#[test]
fn init_resume_route_clears_corrupt_state_fail_soft() {
    let dir = tempdir().expect("tempdir");
    let axhub_dir = dir.path().join(".axhub");
    fs::create_dir_all(&axhub_dir).expect("mkdir .axhub");
    fs::write(axhub_dir.join("init-resume.json"), "{not-json").expect("write corrupt state");

    let (status, json) = run_json(dir.path(), &["init-resume", "route", "--json"]);

    assert_eq!(status, 0);
    assert_eq!(json["route"], "fresh");
    assert_eq!(json["reason"], "state_corrupt");
    assert!(!axhub_dir.join("init-resume.json").exists());
}

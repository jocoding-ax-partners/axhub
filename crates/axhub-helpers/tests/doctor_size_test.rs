// Phase 25 PR 25.6 — `axhub-helpers doctor` deploy-events monitoring.
//
// Each test isolates `XDG_STATE_HOME` to a fresh tempdir so the doctor
// subcommand reads a deterministic deploy-events directory and writes its
// cooldown marker to a sandbox.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_doctor(args: &[&str], state_dir: &std::path::Path) -> Output {
    let mut command = Command::new(bin());
    command
        .args(std::iter::once("doctor").chain(args.iter().copied()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env("XDG_STATE_HOME", state_dir);
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    command.output().unwrap()
}

fn write_deploy_event(dir: &std::path::Path, name: &str, bytes: usize) {
    let target = dir.join(format!("{name}.jsonl"));
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)
        .unwrap();
    let blob = vec![b'a'; bytes];
    f.write_all(&blob).unwrap();
    writeln!(f).unwrap();
}

fn stdout_json(out: &Output) -> serde_json::Value {
    let raw = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&raw).unwrap_or_else(|_| panic!("not JSON: {raw}"))
}

#[test]
fn doctor_with_empty_deploy_events_dir_reports_zero_bytes() {
    let state = tempfile::tempdir().unwrap();
    let out = run_doctor(&["--json"], state.path());
    assert!(out.status.success());
    let json = stdout_json(&out);
    assert_eq!(json["deploy_events_size_bytes"], 0);
    assert_eq!(json["deploy_events_count"], 0);
    assert_eq!(json["over_threshold"], false);
    assert_eq!(json["should_warn"], false);
}

#[test]
fn doctor_reports_actual_size_when_deploy_events_present() {
    let state = tempfile::tempdir().unwrap();
    let dir = state.path().join("axhub-plugin").join("deploy-events");
    write_deploy_event(&dir, "dep-001", 1024);
    write_deploy_event(&dir, "dep-002", 2048);

    let out = run_doctor(&["--json"], state.path());
    let json = stdout_json(&out);
    let size = json["deploy_events_size_bytes"].as_u64().unwrap();
    // Each entry is `bytes` bytes plus one trailing newline.
    assert!((1024 + 2048..=1024 + 2048 + 4).contains(&size));
    assert_eq!(json["deploy_events_count"], 2);
    assert_eq!(json["over_threshold"], false);
}

#[test]
fn doctor_warns_when_over_threshold_and_writes_cooldown() {
    let state = tempfile::tempdir().unwrap();
    let dir = state.path().join("axhub-plugin").join("deploy-events");
    // 100 MB + 1 byte to definitively cross the threshold.
    write_deploy_event(&dir, "dep-huge", 100 * 1024 * 1024 + 1);

    let out = run_doctor(&["--json"], state.path());
    let json = stdout_json(&out);
    assert_eq!(json["over_threshold"], true);
    assert_eq!(json["should_warn"], true);

    // Cooldown marker file must exist.
    let cooldown = state
        .path()
        .join("axhub-plugin")
        .join("doctor-cooldown.json");
    assert!(cooldown.exists(), "cooldown file missing");
}

#[test]
fn doctor_respects_cooldown_within_one_hour() {
    let state = tempfile::tempdir().unwrap();
    let dir = state.path().join("axhub-plugin").join("deploy-events");
    write_deploy_event(&dir, "dep-huge", 100 * 1024 * 1024 + 1);

    // First run → should_warn=true + cooldown written.
    let first = run_doctor(&["--json"], state.path());
    assert_eq!(stdout_json(&first)["should_warn"], true);

    // Second run within cooldown window → should_warn=false.
    let second = run_doctor(&["--json"], state.path());
    assert_eq!(stdout_json(&second)["should_warn"], false);
    assert_eq!(stdout_json(&second)["over_threshold"], true);
}

#[test]
fn doctor_no_cooldown_flag_bypasses_window() {
    let state = tempfile::tempdir().unwrap();
    let dir = state.path().join("axhub-plugin").join("deploy-events");
    write_deploy_event(&dir, "dep-huge", 100 * 1024 * 1024 + 1);

    // Seed cooldown so a default run would skip.
    let _ = run_doctor(&["--json"], state.path());
    let forced = run_doctor(&["--json", "--no-cooldown"], state.path());
    assert_eq!(stdout_json(&forced)["should_warn"], true);
}

#[test]
fn doctor_non_json_mode_prints_korean_warning_line() {
    let state = tempfile::tempdir().unwrap();
    let dir = state.path().join("axhub-plugin").join("deploy-events");
    write_deploy_event(&dir, "dep-huge", 100 * 1024 * 1024 + 1);

    let out = run_doctor(&[], state.path());
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(s.contains("deploy-events"));
    assert!(s.contains("axhub-helpers v"));
    assert!(s.contains("⚠️"));
    assert!(s.contains("100 MB"));
}

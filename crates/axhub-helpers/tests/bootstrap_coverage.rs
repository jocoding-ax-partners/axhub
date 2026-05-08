use axhub_helpers::bootstrap::BOOTSTRAP_RECORD_SCHEMA_VERSION;
use serde_json::json;
use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_bootstrap(args: &[&str], stdin: Option<&str>) -> std::process::Output {
    let mut child = Command::new(bin())
        .arg("bootstrap")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn axhub-helpers bootstrap");
    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }
    child.wait_with_output().expect("wait")
}

#[test]
fn bootstrap_record_rejects_missing_record_event_before_state_lookup() {
    let output = run_bootstrap(&["--record"], None);
    assert_eq!(output.status.code(), Some(64));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("record_event_missing"), "{stdout}");
}

#[test]
fn bootstrap_record_rejects_unknown_flags_before_state_lookup() {
    let output = run_bootstrap(&["--bogus"], None);
    assert_eq!(output.status.code(), Some(64));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unknown_option:--bogus"), "{stdout}");
}

#[test]
fn bootstrap_record_rejects_missing_schema_version() {
    let output = run_bootstrap(&["--record", "apps_create"], Some("{}"));
    assert_eq!(output.status.code(), Some(64));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("record_schema_version_missing"), "{stdout}");
}

#[test]
fn bootstrap_record_rejects_schema_version_mismatch() {
    let envelope = json!({
        "schema_version": "wrong/v1",
        "pending_action_id": "p",
        "pending_action_hash": "h",
        "command_argv": [],
        "exit_code": 0,
        "stdout_json": {}
    });
    let output = run_bootstrap(&["--record", "apps_create"], Some(&envelope.to_string()));
    assert_eq!(output.status.code(), Some(64));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("record_schema_version_mismatch"),
        "{stdout}"
    );
}

#[test]
fn bootstrap_record_rejects_duplicate_without_pending_action() {
    let envelope = json!({
        "schema_version": BOOTSTRAP_RECORD_SCHEMA_VERSION,
        "pending_action_id": "p",
        "pending_action_hash": "h",
        "command_argv": [],
        "exit_code": 0,
        "stdout_json": {}
    });
    let output = run_bootstrap(&["--record", "apps_create"], Some(&envelope.to_string()));
    assert_eq!(output.status.code(), Some(64));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("record_duplicate_or_no_pending_action"),
        "{stdout}"
    );
}

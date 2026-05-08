//! Phase 2 — `version --quiet` behavior.
//!
//! Spec: `.plan/deploy-time-reduction/phase-2-helper-batch-telemetry-inprocess.md` §4.2.
//!
//! `axhub-helpers --version --quiet` is invoked by the SessionStart hook on
//! macOS to warm the Gatekeeper / notarization cache. The hook expects:
//!   - exit 0
//!   - empty stdout (no version line) so the JSON-only hook output is not
//!     polluted
//!   - empty stderr (best-effort silence)

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

#[test]
fn version_without_quiet_prints_version_line() {
    let out = Command::new(BIN)
        .arg("--version")
        .output()
        .expect("spawn axhub-helpers --version");
    assert!(out.status.success(), "exit code {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("axhub-helpers "),
        "stdout did not start with version banner: {stdout}"
    );
    assert!(stdout.contains("schema v0"));
}

#[test]
fn version_with_quiet_suppresses_all_output() {
    let out = Command::new(BIN)
        .args(["--version", "--quiet"])
        .output()
        .expect("spawn axhub-helpers --version --quiet");
    assert!(out.status.success(), "exit code {:?}", out.status.code());
    assert!(
        out.stdout.is_empty(),
        "stdout should be empty, got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        out.stderr.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn version_quiet_argument_order_does_not_matter() {
    let out = Command::new(BIN)
        .args(["version", "--quiet"])
        .output()
        .expect("spawn axhub-helpers version --quiet");
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

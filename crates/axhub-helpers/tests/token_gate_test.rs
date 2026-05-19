// Phase 1.1 token-gate regression tests — Rust port of hooks/token-freshness-gate.sh.
//
// Covers:
//   - kill switch (AXHUB_DISABLE_HOOKS / AXHUB_DISABLE_HOOK / DISABLE_AXHUB)
//   - AXHUB_AUTH_BG_REFRESH=0 silent skip
//   - fresh mtime → exit 0
//   - stale mtime → polling timeout → inline auth probe → exit 65 UNAUTHORIZED
//   - stale mtime → refresh during polling → exit 0
//   - missing token file → inline check
//   - AXHUB_GATE_AUTH_PROBE shellwords parse (POSIX) + Command::new (NOT eval)
//
// All tests inject AXHUB_GATE_FAKE_NOW + AXHUB_TOKEN_PATH + AXHUB_GATE_AUTH_PROBE
// to exercise the gate without live OAuth flow. POLL_INTERVAL/POLL_ITERATIONS are
// shortened so the test suite stays under the default Rust test timeout.

use std::fs;
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn run_gate(envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin());
    cmd.arg("token-gate");
    // Strip the host-inherited kill-switch envs so the test environment
    // can't accidentally short-circuit unrelated cases.
    for key in [
        "AXHUB_DISABLE_HOOKS",
        "AXHUB_DISABLE_HOOK",
        "DISABLE_AXHUB",
        "AXHUB_AUTH_BG_REFRESH",
        "AXHUB_TOKEN_PATH",
        "AXHUB_GATE_FAKE_NOW",
        "AXHUB_GATE_POLL_INTERVAL",
        "AXHUB_GATE_POLL_ITERATIONS",
        "AXHUB_GATE_AUTH_PROBE",
    ] {
        cmd.env_remove(key);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn probe_unauthorized() -> &'static str {
    // Probe that always emits an empty JSON object — does NOT contain
    // `"user_email"` substring, so cmd_token_gate treats it as UNAUTHORIZED.
    if cfg!(windows) {
        "cmd /c echo {}"
    } else {
        "printf {}"
    }
}

fn probe_authorized() -> &'static str {
    // Probe that emits literal `"user_email":"u"` (quotes included) so
    // cmd_token_gate's `stdout.contains("\"user_email\"")` check matches.
    // We route through `sh -c` (POSIX) or `cmd /c` (Windows) so the inner
    // shell expands the backslash-escaped quotes before printf/echo runs.
    if cfg!(windows) {
        r#"cmd /c echo "user_email":"u""#
    } else {
        // shlex POSIX 가 single-quoted body 를 literal 로 보존 →
        // bash -c 가 double-quoted `\"` 를 literal quote 로 expand →
        // printf 가 `"user_email":"u"` 그대로 stdout 출력.
        r#"bash -c 'printf "\"user_email\":\"u\""'"#
    }
}

fn touch_mtime(path: &std::path::Path, secs_since_epoch: i64) {
    // Set the file mtime explicitly so tests can simulate fresh / stale tokens
    // regardless of when the file was actually created.
    let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(secs_since_epoch.max(0) as u64);
    let f = std::fs::File::open(path).unwrap();
    f.set_modified(mtime).unwrap();
}

#[test]
fn kill_switch_disable_hooks_short_circuits() {
    let out = run_gate(&[("AXHUB_DISABLE_HOOKS", "1")]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn kill_switch_disable_hook_csv_short_circuits() {
    let out = run_gate(&[("AXHUB_DISABLE_HOOK", "other,token-freshness-gate")]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn kill_switch_legacy_disable_axhub_short_circuits() {
    let out = run_gate(&[("DISABLE_AXHUB", "1")]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn auth_bg_refresh_zero_silent_skips() {
    let out = run_gate(&[("AXHUB_AUTH_BG_REFRESH", "0")]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn fresh_token_exits_zero_without_probe() {
    let tmp = tempfile::tempdir().unwrap();
    let token_path = tmp.path().join("token");
    fs::write(&token_path, "fresh").unwrap();
    let now = now_secs();
    // Set mtime to "now" — mtime > session_ts (now - 30) so cmd_token_gate
    // takes the fresh branch and exits 0 without ever spawning the probe.
    touch_mtime(&token_path, now);
    let out = run_gate(&[
        ("AXHUB_TOKEN_PATH", token_path.to_str().unwrap()),
        ("AXHUB_GATE_FAKE_NOW", &now.to_string()),
        // Probe set to something that would exit non-zero if invoked, to
        // double-check that the fresh branch never spawned it.
        ("AXHUB_GATE_AUTH_PROBE", "false"),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("token mtime > session_ts, fresh"),
        "expected fresh-path log, got: {stderr}"
    );
}

#[test]
fn stale_token_polling_timeout_invokes_unauthorized_probe_exits_65() {
    let tmp = tempfile::tempdir().unwrap();
    let token_path = tmp.path().join("token");
    fs::write(&token_path, "stale").unwrap();
    let now = now_secs();
    // Set mtime well before session_ts (now - 30) so the fresh check fails.
    touch_mtime(&token_path, now - 3600);
    let out = run_gate(&[
        ("AXHUB_TOKEN_PATH", token_path.to_str().unwrap()),
        ("AXHUB_GATE_FAKE_NOW", &now.to_string()),
        ("AXHUB_GATE_POLL_INTERVAL", "0"), // sleep 0 → loop spins immediately
        ("AXHUB_GATE_POLL_ITERATIONS", "2"),
        ("AXHUB_GATE_AUTH_PROBE", probe_unauthorized()),
    ]);
    assert_eq!(out.status.code(), Some(65));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("auth UNAUTHORIZED"),
        "expected UNAUTHORIZED log, got: {stderr}"
    );
}

#[test]
fn stale_token_polling_picks_up_refreshed_mtime_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let token_path = tmp.path().join("token");
    fs::write(&token_path, "stale").unwrap();
    let now = now_secs();
    // Start with stale mtime so fresh branch fails.
    touch_mtime(&token_path, now - 3600);
    // Use a small poll interval (1s) and spawn a background thread that
    // refreshes the mtime mid-polling. The gate process re-stats the file
    // each iteration and should exit 0 when mtime crosses session_ts.
    let path_clone = token_path.clone();
    let _refresher = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(800));
        touch_mtime(&path_clone, now_secs());
    });
    let out = run_gate(&[
        ("AXHUB_TOKEN_PATH", token_path.to_str().unwrap()),
        ("AXHUB_GATE_FAKE_NOW", &now.to_string()),
        ("AXHUB_GATE_POLL_INTERVAL", "1"),
        ("AXHUB_GATE_POLL_ITERATIONS", "4"),
        // Probe set to something that would exit non-zero if invoked, to
        // double-check that polling detected the refresh.
        ("AXHUB_GATE_AUTH_PROBE", "false"),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn missing_token_file_falls_through_to_inline_check_unauthorized() {
    let tmp = tempfile::tempdir().unwrap();
    let token_path = tmp.path().join("missing");
    // Do NOT create the file.
    let out = run_gate(&[
        ("AXHUB_TOKEN_PATH", token_path.to_str().unwrap()),
        ("AXHUB_GATE_FAKE_NOW", &now_secs().to_string()),
        ("AXHUB_GATE_AUTH_PROBE", probe_unauthorized()),
    ]);
    assert_eq!(out.status.code(), Some(65));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("token file missing"),
        "expected token file missing log, got: {stderr}"
    );
}

#[test]
fn inline_auth_probe_with_user_email_match_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let token_path = tmp.path().join("missing");
    let out = run_gate(&[
        ("AXHUB_TOKEN_PATH", token_path.to_str().unwrap()),
        ("AXHUB_GATE_FAKE_NOW", &now_secs().to_string()),
        ("AXHUB_GATE_AUTH_PROBE", probe_authorized()),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn malformed_probe_fails_open_with_exit_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let token_path = tmp.path().join("missing");
    let out = run_gate(&[
        ("AXHUB_TOKEN_PATH", token_path.to_str().unwrap()),
        ("AXHUB_GATE_FAKE_NOW", &now_secs().to_string()),
        // Single trailing backslash — POSIX shlex returns None / empty on
        // malformed input. cmd_token_gate should fail-open with exit 0.
        ("AXHUB_GATE_AUTH_PROBE", "  "),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("shellwords parse failed") || stderr.contains("inline auth status check"),
        "expected shellwords fail-open log, got: {stderr}"
    );
}

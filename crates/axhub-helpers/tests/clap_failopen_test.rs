//! Phase 2 (clap scaffold) — fail-open on parse failure (FR-002 / SC-003).
//!
//! 무인자/unknown-ignore hook 명령은 잘못된 flag 를 줘도 **exit 0** 이어야 해요.
//! 전환기엔 passthrough→legacy(unknown flag 무시)가, typed 이관 후엔
//! classify()→FailOpenHook→handle_parse_error 가 이 계약을 지켜요.
//!
//! 참고: flag-bearing hook `state-update` 의 malformed→64 보존(parity guard)은
//! git-repo 컨텍스트가 필요해서 US1(T018, phase26_quality_cli 하니스)에서 검증해요.

use std::io::Write;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

fn run_stdin(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // ignore BrokenPipe — child may exit before draining stdin
    let _ = child.stdin.as_mut().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

#[test]
fn classify_exit_failopen_on_bad_flag() {
    let out = run_stdin(&["classify-exit", "--bogus-unknown-flag"], "");
    assert_eq!(
        out.status.code(),
        Some(0),
        "classify-exit must fail-open (exit 0) on unknown flag"
    );
}

#[test]
fn tdd_inject_failopen_on_bad_flag() {
    let out = run_stdin(&["tdd-inject", "--bogus-unknown-flag"], "");
    assert_eq!(
        out.status.code(),
        Some(0),
        "tdd-inject must fail-open (exit 0) on unknown flag"
    );
}

#[test]
fn autowire_statusline_failopen_on_bad_flag() {
    let out = run_stdin(&["autowire-statusline", "--bogus-unknown-flag"], "");
    assert_eq!(
        out.status.code(),
        Some(0),
        "autowire-statusline must fail-open (exit 0) on clap parse failures"
    );
}

#[test]
fn verify_deploy_artifact_failopen_on_bad_flag() {
    let out = run_stdin(&["verify-deploy-artifact", "--bogus-unknown-flag"], "");
    assert_eq!(
        out.status.code(),
        Some(0),
        "verify-deploy-artifact must fail-open (exit 0) on clap parse failures"
    );
}

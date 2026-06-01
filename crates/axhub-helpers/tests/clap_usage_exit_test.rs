//! Phase 2 (clap scaffold) — usage-error exit code remap (D4 / SC-006 / FR-005).
//!
//! 알 수 없는 subcommand·subcommand 미지정은 기존 **exit 64** 로 끝나야 해요
//! (clap 기본 usage-error exit 2 가 아니라). 전환기엔 passthrough→legacy `_` arm 이,
//! typed 이관 후엔 clap 에러 remap 이 이 계약을 지켜요.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

#[test]
fn unknown_subcommand_exits_64() {
    let out = Command::new(BIN).arg("bogus-subcommand").output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(64),
        "unknown subcommand must exit 64 (not clap default 2)"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unknown subcommand"),
        "stderr must keep legacy 'unknown subcommand' wording during transition"
    );
}

#[test]
fn no_subcommand_exits_64() {
    let out = Command::new(BIN).output().unwrap();
    assert_eq!(out.status.code(), Some(64), "no subcommand must exit 64");
    assert!(
        !out.stderr.is_empty(),
        "no-subcommand must print USAGE to stderr"
    );
}

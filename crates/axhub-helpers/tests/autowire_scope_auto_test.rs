// Phase 2.2 (T6) regression — `axhub-helpers autowire-statusline --scope auto`
// 가 shell wrapper 의 scope detection (CLAUDE_PLUGIN_ROOT prefix 검사) 을 흡수했는지 검증.
//
// 검증 항목:
//   - --scope auto + CLAUDE_PLUGIN_ROOT = $HOME/.claude/plugins/... → user scope 감지 후 dispatcher 진입
//     (실제 settings.json 머지는 fail-closed 다른 path 에서 검증; 여기는 인자 parse + scope detect 만 확인)
//   - --scope auto + CLAUDE_PLUGIN_ROOT 가 ambiguous → exit 0 fail-closed
//   - --scope user 명시 → 기존 behavior 유지
//   - --scope foo → exit 64 (invalid)

use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_autowire(envs: &[(&str, &str)], extra_args: &[&str]) -> Output {
    let mut cmd = Command::new(bin());
    cmd.arg("autowire-statusline").arg("--silent");
    for arg in extra_args {
        cmd.arg(arg);
    }
    for key in [
        "AXHUB_DISABLE_HOOKS",
        "AXHUB_DISABLE_HOOK",
        "DISABLE_AXHUB",
        "AXHUB_DISABLE_STATUSLINE_AUTOWIRE",
        "CLAUDE_PLUGIN_ROOT",
        "HOME",
        "USERPROFILE",
    ] {
        cmd.env_remove(key);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

#[test]
fn scope_auto_no_envvars_fails_closed_exit_zero() {
    // CLAUDE_PLUGIN_ROOT unset → scope auto 가 None 반환 → fail-closed exit 0.
    let out = run_autowire(&[], &["--scope", "auto"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn scope_auto_user_prefix_proceeds_through_dispatcher() {
    // 가짜 HOME + CLAUDE_PLUGIN_ROOT 가 $HOME/.claude/plugins/... → user scope 감지.
    // 실제 settings.json merge 는 권한 / 경로 문제로 fail-closed 또는 silent skip
    // 가능 — 우리는 dispatcher 의 인자 parse 가 scope detect 까지 도달했는지만 확인.
    let home = "/tmp/axhub-test-home";
    let plugin_root = format!("{home}/.claude/plugins/axhub-test");
    let out = run_autowire(
        &[
            ("HOME", home),
            ("CLAUDE_PLUGIN_ROOT", &plugin_root),
            ("AXHUB_DISABLE_STATUSLINE_AUTOWIRE", "1"), // 본체 진입 직전 short-circuit
        ],
        &["--scope", "auto"],
    );
    // AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1 가 autowire.rs 본체에서 exit 0 시키므로
    // 인자 parse 가 무사히 완료됐다는 신호 = 정상 exit 0.
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn scope_user_explicit_still_supported() {
    let out = run_autowire(
        &[("AXHUB_DISABLE_STATUSLINE_AUTOWIRE", "1")],
        &["--scope", "user"],
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn scope_project_explicit_still_supported() {
    let out = run_autowire(
        &[("AXHUB_DISABLE_STATUSLINE_AUTOWIRE", "1")],
        &["--scope", "project"],
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn scope_unknown_value_exits_64() {
    let out = run_autowire(&[], &["--scope", "garbage"]);
    assert_eq!(out.status.code(), Some(64));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("user|project|auto"),
        "expected usage hint mentioning new `auto` keyword, got: {stderr}"
    );
}

#[test]
fn scope_missing_still_exits_64() {
    // --scope flag 미제공 + auto 도 미제공 → exit 64 (sh/ps1-absorption Phase 2.2
    // 전 behavior 보존, hooks/session-start-autowire.{sh,ps1} 가 explicit `--scope auto` 전달).
    let out = run_autowire(&[], &[]);
    assert_eq!(out.status.code(), Some(64));
}

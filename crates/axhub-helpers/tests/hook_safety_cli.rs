// Phase 25 PR 25.2 — Hook safety CLI integration tests.
//
// Verifies that the axhub hook entry points (session-start, prompt-route,
// classify-exit) honor `AXHUB_DISABLE_HOOKS`,
// `AXHUB_DISABLE_HOOK=<csv>`, and the legacy `DISABLE_AXHUB` alias per the
// kill-switch precedence rules in `docs/HOOKS.md`. Each test spawns the
// helper binary the same way `cli_e2e.rs` does so we exercise the real
// dispatch path, not the unit-test mocks.

use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_stdin(args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Clear inherited env so host shell vars (e.g. AXHUB_TELEMETRY) don't
    // leak into our matrix and falsify "default" baselines.
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    // ast-validate hook block-mode opt-in must not leak from the host shell into
    // warn-only baselines.
    command.env_remove("AXHUB_AST_VALIDATE");
    // session-start's drift-cache warm only fires when CLAUDE_PLUGIN_ROOT marks a
    // real plugin session; removing it here keeps the suite hermetic (never spawns
    // a network fetch) even if a dev runs `cargo test` with it exported.
    command.env_remove("CLAUDE_PLUGIN_ROOT");

    // Sandbox audit + telemetry writes for prompt-route. Both XDG_STATE_HOME and
    // XDG_CACHE_HOME are isolated so prompt-route never reads the developer's real
    // plugin-latest.json / cli-latest.json (which would inject a real drift nudge
    // and falsify systemMessage assertions).
    let state_dir = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    command.env("XDG_STATE_HOME", state_dir.path());
    command.env("XDG_CACHE_HOME", cache_dir.path());
    command.env("AXHUB_NO_AUDIT", "1");

    for (k, v) in envs {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    match child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::BrokenPipe => {}
        Err(err) => panic!("failed to write child stdin: {err}"),
    }
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

// --- session-start --------------------------------------------------------

#[test]
fn session_start_baseline_emits_welcome_lines() {
    let out = run_stdin(&["session-start"], "", &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        s.contains("axhub 준비됐어요"),
        "baseline session-start should emit welcome line, got: {s}"
    );
}

#[test]
fn session_start_global_kill_switch_skips_silently() {
    let out = run_stdin(&["session-start"], "", &[("AXHUB_DISABLE_HOOKS", "1")]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert_eq!(
        s.trim(),
        "{}",
        "expected empty JSON pass signal, got: {s:?}"
    );
}

#[test]
fn session_start_per_hook_kill_switch_skips() {
    let out = run_stdin(
        &["session-start"],
        "",
        &[("AXHUB_DISABLE_HOOK", "session-start , other")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn session_start_per_hook_csv_without_match_runs_normally() {
    let out = run_stdin(
        &["session-start"],
        "",
        &[("AXHUB_DISABLE_HOOK", "prompt-route,classify-exit")],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("axhub 준비됐어요"));
}

// --- legacy alias ---------------------------------------------------------

#[test]
fn legacy_disable_axhub_skips_with_deprecation_warning() {
    let out = run_stdin(&["session-start"], "", &[("DISABLE_AXHUB", "1")]);
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
    let err = stderr(&out);
    assert!(
        err.contains("DISABLE_AXHUB") && err.contains("deprecated"),
        "expected legacy deprecation warning on stderr, got: {err:?}"
    );
}

#[test]
fn legacy_alias_loses_to_canonical_global() {
    // global canonical present → skip path, no spurious legacy warning.
    let out = run_stdin(
        &["session-start"],
        "",
        &[("AXHUB_DISABLE_HOOKS", "1"), ("DISABLE_AXHUB", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
    // canonical short-circuits before legacy check → no warning.
    let err = stderr(&out);
    assert!(
        !err.contains("deprecated"),
        "canonical should short-circuit legacy warning, got stderr: {err:?}"
    );
}

// --- prompt-route ---------------------------------------------------------

#[test]
fn prompt_route_kill_switch_skips_audit() {
    let out = run_stdin(
        &["prompt-route"],
        r#"{"prompt":"axhub 배포해줘"}"#,
        &[("AXHUB_DISABLE_HOOKS", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

// --- classify-exit --------------------------------------------------------

#[test]
fn classify_exit_kill_switch_skips() {
    let out = run_stdin(
        &["classify-exit"],
        r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":64,"stdout":"oops"}}"#,
        &[("AXHUB_DISABLE_HOOKS", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn classify_exit_per_hook_csv_skips() {
    let out = run_stdin(
        &["classify-exit"],
        r#"{"tool_input":{"command":"axhub deploy create"},"tool_response":{"exit_code":64,"stdout":"oops"}}"#,
        &[("AXHUB_DISABLE_HOOK", "classify-exit")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

// --- plugin-drift (proactive update nudge) --------------------------------
//
// End-to-end: seed a fresh "newer version available" cache, then run
// prompt-route. The nudge rides UserPromptSubmit additionalContext (D4) and the
// user-facing systemMessage fallback (live QA found agents may ignore the
// context-only instruction). It is gated by AXHUB_DISABLE_HOOK=plugin-drift.
// Tempdirs isolate the per-version marker so neither test leaks state into the
// other.

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn run_prompt_route_with_fresh_newer_cache(extra_env: &[(&str, &str)]) -> Output {
    let cache_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let plugin_cache = cache_dir.path().join("axhub-plugin");
    std::fs::create_dir_all(&plugin_cache).unwrap();
    // "99.0.0" is unconditionally newer than the compiled CARGO_PKG_VERSION.
    std::fs::write(
        plugin_cache.join("plugin-latest.json"),
        format!(r#"{{"latest":"99.0.0","fetched_at":{}}}"#, now_secs()),
    )
    .unwrap();

    let mut command = Command::new(bin());
    command
        .args(["prompt-route"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    command.env_remove("CI");
    command.env_remove("CLAUDE_NON_INTERACTIVE");
    command.env("XDG_CACHE_HOME", cache_dir.path());
    command.env("XDG_STATE_HOME", state_dir.path());
    command.env("AXHUB_NO_AUDIT", "1");
    for (k, v) in extra_env {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    let _ = child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"prompt":"hello"}"#);
    // tempdirs stay alive until wait_with_output returns (child has exited).
    child.wait_with_output().unwrap()
}

#[test]
fn plugin_drift_nudge_fires_when_newer_version_cached() {
    let out = run_prompt_route_with_fresh_newer_cache(&[]);
    assert!(out.status.success());
    let s = stdout(&out);
    let json: serde_json::Value = serde_json::from_str(&s).unwrap();
    let ctx = json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    let msg = json["systemMessage"].as_str().unwrap();
    assert!(
        ctx.contains("플러그인 새 버전")
            && ctx.contains("에이전트 필수 동작")
            && ctx.contains("AskUserQuestion")
            && ctx.contains("그만 볼래요"),
        "expected drift nudge with opt-out option in additionalContext, got: {s}"
    );
    assert!(
        msg.contains("플러그인 새 버전")
            && msg.contains("업데이트할까요?")
            && msg.contains("업데이트할래요")
            && msg.contains("지금은 그대로")
            && msg.contains("그만 볼래요"),
        "expected user-facing plugin drift systemMessage, got: {s}"
    );
}

#[test]
fn plugin_drift_kill_switch_suppresses_nudge() {
    let out = run_prompt_route_with_fresh_newer_cache(&[("AXHUB_DISABLE_HOOK", "plugin-drift")]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        !s.contains("플러그인 새 버전"),
        "AXHUB_DISABLE_HOOK=plugin-drift must suppress the nudge, got: {s}"
    );
}

// Run prompt-route with a fresh newer cache AND a pre-seeded per-version snooze
// marker, passing `session_id` in the payload. Proves the re-surface contract
// end-to-end: the marker no longer silences the nudge forever — only within the
// same session inside the snooze window.
fn run_prompt_route_with_marker_and_session(
    marker_session: &str,
    marker_age_secs: u64,
    payload_session: &str,
) -> Output {
    let cache_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let plugin_cache = cache_dir.path().join("axhub-plugin");
    std::fs::create_dir_all(&plugin_cache).unwrap();
    std::fs::write(
        plugin_cache.join("plugin-latest.json"),
        format!(r#"{{"latest":"99.0.0","fetched_at":{}}}"#, now_secs()),
    )
    .unwrap();
    // Pre-seed the snooze marker for v99.0.0 in the state dir.
    let state_plugin = state_dir.path().join("axhub-plugin");
    std::fs::create_dir_all(&state_plugin).unwrap();
    std::fs::write(
        state_plugin.join(".plugin-drift-nudged-v99.0.0"),
        format!(
            r#"{{"session":"{marker_session}","at":{}}}"#,
            now_secs() - marker_age_secs
        ),
    )
    .unwrap();

    let mut command = Command::new(bin());
    command
        .args(["prompt-route"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    command.env_remove("CI");
    command.env_remove("CLAUDE_NON_INTERACTIVE");
    command.env("XDG_CACHE_HOME", cache_dir.path());
    command.env("XDG_STATE_HOME", state_dir.path());
    command.env("AXHUB_NO_AUDIT", "1");
    let mut child = command.spawn().unwrap();
    let payload = format!(r#"{{"prompt":"hello","session_id":"{payload_session}"}}"#);
    let _ = child.stdin.as_mut().unwrap().write_all(payload.as_bytes());
    child.wait_with_output().unwrap()
}

#[test]
fn plugin_drift_renudges_in_a_new_session() {
    // Already nudged in "session-A"; a NEW session "session-B" re-surfaces it.
    let out = run_prompt_route_with_marker_and_session("session-A", 60, "session-B");
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        s.contains("플러그인 새 버전"),
        "a new session must re-surface the plugin drift nudge despite the marker, got: {s}"
    );
}

#[test]
fn plugin_drift_snoozes_within_the_same_session() {
    // Same session, marker still fresh (60s old) → suppressed (turn-cap intact).
    let out = run_prompt_route_with_marker_and_session("session-A", 60, "session-A");
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        !s.contains("플러그인 새 버전"),
        "a fresh same-session marker must snooze the plugin drift nudge, got: {s}"
    );
}

// --- cli-drift (proactive CLI binary update nudge) ------------------------
//
// End-to-end: seed a fresh `cli-latest.json` (backend has_update=true), then run
// prompt-route. Separate channel from plugin-drift — its own cache file, marker,
// opt-out, and kill switch (AXHUB_DISABLE_HOOK=cli-drift). Verifies the turn-cap
// (plugin priority) and the update-check-intent suppression (E3 / CE-3).

// Seed caches into the given dirs and run prompt-route once. Taking the dir
// paths (not fresh TempDirs) lets a caller drive multiple turns that SHARE state
// — required to exercise the cross-turn marker yield (plugin turn 1 → CLI turn 2).
fn run_prompt_route_in(
    cache_root: &std::path::Path,
    state_root: &std::path::Path,
    plugin_latest: Option<&str>,
    cli_cache: Option<&str>,
    prompt: &str,
    extra_env: &[(&str, &str)],
) -> Output {
    let plugin_cache = cache_root.join("axhub-plugin");
    std::fs::create_dir_all(&plugin_cache).unwrap();
    if let Some(latest) = plugin_latest {
        std::fs::write(
            plugin_cache.join("plugin-latest.json"),
            format!(r#"{{"latest":"{latest}","fetched_at":{}}}"#, now_secs()),
        )
        .unwrap();
    }
    if let Some(cli_json) = cli_cache {
        std::fs::write(plugin_cache.join("cli-latest.json"), cli_json).unwrap();
    }

    let mut command = Command::new(bin());
    command
        .args(["prompt-route"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    command.env_remove("CI");
    command.env_remove("CLAUDE_NON_INTERACTIVE");
    command.env("XDG_CACHE_HOME", cache_root);
    command.env("XDG_STATE_HOME", state_root);
    command.env("AXHUB_NO_AUDIT", "1");
    for (k, v) in extra_env {
        command.env(k, v);
    }
    let mut child = command.spawn().unwrap();
    let payload = format!(r#"{{"prompt":"{prompt}"}}"#);
    let _ = child.stdin.as_mut().unwrap().write_all(payload.as_bytes());
    child.wait_with_output().unwrap()
}

fn run_prompt_route_with_caches(
    plugin_latest: Option<&str>,
    cli_cache: Option<&str>,
    prompt: &str,
    extra_env: &[(&str, &str)],
) -> Output {
    let cache_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    run_prompt_route_in(
        cache_dir.path(),
        state_dir.path(),
        plugin_latest,
        cli_cache,
        prompt,
        extra_env,
    )
}

fn fresh_cli_cache(has_update: bool, disabled: bool) -> String {
    format!(
        r#"{{"current":"0.18.1","latest":"0.18.2","has_update":{has_update},"disabled":{disabled},"fetched_at":{}}}"#,
        now_secs()
    )
}

#[test]
fn cli_drift_nudge_fires_when_cli_update_cached() {
    let out = run_prompt_route_with_caches(None, Some(&fresh_cli_cache(true, false)), "hello", &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    let json: serde_json::Value = serde_json::from_str(&s).unwrap();
    let ctx = json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    let msg = json["systemMessage"].as_str().unwrap();
    assert!(
        ctx.contains("CLI 새 버전 알림")
            && ctx.contains("update 스킬")
            && ctx.contains("cli-drift-optout"),
        "expected CLI drift nudge in additionalContext, got: {s}"
    );
    assert!(
        msg.contains("axhub CLI 새 버전이 나왔어요"),
        "expected user-facing CLI drift systemMessage, got: {s}"
    );
}

#[test]
fn cli_drift_kill_switch_suppresses_nudge() {
    let out = run_prompt_route_with_caches(
        None,
        Some(&fresh_cli_cache(true, false)),
        "hello",
        &[("AXHUB_DISABLE_HOOK", "cli-drift")],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        !s.contains("axhub CLI 새 버전"),
        "AXHUB_DISABLE_HOOK=cli-drift must suppress the nudge, got: {s}"
    );
}

#[test]
fn cli_drift_suppressed_when_cli_autoupdate_disabled() {
    let out = run_prompt_route_with_caches(None, Some(&fresh_cli_cache(true, true)), "hello", &[]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        !s.contains("axhub CLI 새 버전"),
        "disabled:true (AXHUB_DISABLE_AUTOUPDATE) must suppress the nudge, got: {s}"
    );
}

#[test]
fn both_drift_emits_one_unified_nudge() {
    // When BOTH channels have a fresh, newer cache, the user sees ONE unified
    // prompt instead of two separate plugin/CLI nudges (the old turn-cap rotated
    // them across turns). Both per-version markers are stamped.
    let out = run_prompt_route_with_caches(
        Some("99.0.0"),
        Some(&fresh_cli_cache(true, false)),
        "hello",
        &[],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        s.contains("업데이트가 두 가지 있어요"),
        "both updates must fold into one unified nudge, got: {s}"
    );
    assert!(
        s.contains("플러그인") && s.contains("CLI"),
        "unified nudge must name both channels, got: {s}"
    );
    // Not the two separate single-channel nudges.
    assert!(
        !s.contains("플러그인 새 버전") && !s.contains("axhub CLI 새 버전"),
        "must not emit the two separate single-channel nudges, got: {s}"
    );
}

#[test]
fn cli_drift_suppressed_when_prompt_is_update_check_intent() {
    // The reactive update-check path owns the turn when the user is already
    // asking about updates — the proactive CLI nudge must not double up.
    let out = run_prompt_route_with_caches(
        None,
        Some(&fresh_cli_cache(true, false)),
        "업데이트 확인해줘",
        &[],
    );
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(
        !s.contains("axhub CLI 새 버전"),
        "update-check intent must suppress the proactive CLI nudge, got: {s}"
    );
}

#[test]
fn both_drift_unified_once_then_snoozed_same_session() {
    // With BOTH channels drifting and a SHARED state dir (markers persist), turn
    // 1 fires ONE unified nudge and stamps both per-version markers; turn 2 (same
    // session) finds both snoozed → no drift nudge at all. The unified prompt
    // re-surfaces only in a new session.
    let cache_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cli = fresh_cli_cache(true, false);

    let turn1 = stdout(&run_prompt_route_in(
        cache_dir.path(),
        state_dir.path(),
        Some("99.0.0"),
        Some(&cli),
        "hello",
        &[],
    ));
    assert!(
        turn1.contains("업데이트가 두 가지 있어요"),
        "turn 1 = one unified nudge, got: {turn1}"
    );

    let turn2 = stdout(&run_prompt_route_in(
        cache_dir.path(),
        state_dir.path(),
        Some("99.0.0"),
        Some(&cli),
        "hello",
        &[],
    ));
    assert!(
        !turn2.contains("업데이트가 두 가지 있어요")
            && !turn2.contains("플러그인 새 버전")
            && !turn2.contains("axhub CLI 새 버전"),
        "turn 2 same session must be snoozed (no drift nudge), got: {turn2}"
    );
}

// --- ast-validate (PostToolUse static validator hook) ---------------------
//
// The hook reads a PostToolUse payload on stdin, extracts the edited file
// path, and statically validates it. Contract: ALWAYS exit 0 (fail-open),
// warn-only by default (systemMessage), block mode adds additionalContext.

fn fixture(rel: &str) -> String {
    format!("{}/tests/fixtures/ast-validate/{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn ast_validate_payload(rel: &str) -> String {
    format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, fixture(rel))
}

#[test]
fn ast_validate_hook_warn_only_emits_systemmessage_on_block() {
    let out = run_stdin(&["ast-validate"], &ast_validate_payload("node/bad.ts"), &[]);
    assert!(out.status.success(), "hook must exit 0 (fail-open)");
    let s = stdout(&out);
    let json: serde_json::Value = serde_json::from_str(s.trim()).unwrap();
    assert!(
        json["systemMessage"]
            .as_str()
            .unwrap_or("")
            .contains("AST validator"),
        "warn-only must surface a systemMessage, got: {s}"
    );
    assert!(
        json.get("hookSpecificOutput").is_none(),
        "warn-only must NOT inject additionalContext, got: {s}"
    );
}

#[test]
fn ast_validate_hook_block_mode_adds_agent_context() {
    let out = run_stdin(
        &["ast-validate"],
        &ast_validate_payload("node/bad.ts"),
        &[("AXHUB_AST_VALIDATE", "block")],
    );
    assert!(out.status.success(), "hook must exit 0 even in block mode");
    let s = stdout(&out);
    let json: serde_json::Value = serde_json::from_str(s.trim()).unwrap();
    assert!(
        json["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap_or("")
            .contains("고쳐주세요"),
        "block mode must inject corrective additionalContext, got: {s}"
    );
}

#[test]
fn ast_validate_hook_clean_file_stays_silent() {
    let out = run_stdin(&["ast-validate"], &ast_validate_payload("node/good.ts"), &[]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out).trim(),
        "",
        "clean file (advisory only) must produce no hook output"
    );
}

#[test]
fn ast_validate_hook_global_kill_switch_silent() {
    let out = run_stdin(
        &["ast-validate"],
        &ast_validate_payload("node/bad.ts"),
        &[("AXHUB_DISABLE_HOOKS", "1")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "");
}

#[test]
fn ast_validate_hook_per_hook_kill_switch_silent() {
    let out = run_stdin(
        &["ast-validate"],
        &ast_validate_payload("node/bad.ts"),
        &[("AXHUB_DISABLE_HOOK", "ast-validate,other")],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "");
}

#[test]
fn ast_validate_hook_unsupported_path_silent() {
    let out = run_stdin(
        &["ast-validate"],
        r#"{"tool_input":{"file_path":"/tmp/readme.md"}}"#,
        &[],
    );
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "");
}

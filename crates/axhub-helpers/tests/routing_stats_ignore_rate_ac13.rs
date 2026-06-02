//! AC-13 (spec 006 §82, exit `audit-trail-operational`): the `routing-stats`
//! skill reports the **non-axhub ignore rate** from audit data.
//!
//! AC-12 already wired `cmd_prompt_route` to persist the shared routing decision
//! per prompt and `cmd_routing_stats` to surface `decision_counts` / `ignore_rate`.
//! The AC-12 e2e (`cli_routing_stats_reports_decision_breakdown`) exercises only
//! `yield` + `axhub` records, so the *non-axhub `ignore`* value path — the actual
//! "zero-footprint pass-through" signal this AC is about — stays unasserted.
//!
//! `ignore` is the one decision no keyword can force: it requires bare NL + marker
//! `Absent` (see `routing::decide_from_flags` rule `e`). So this test drives a real
//! end-to-end ignore by running `prompt-route` from a **non-marker repo cwd**
//! (`.git`, no `axhub.yaml`), then asserts `routing-stats` reports the ignore rate
//! computed from those audit lines.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

/// Minimal fake `axhub` so `run_preflight()` inside `prompt-route` never shells
/// out to a real binary. Mirrors `cli_e2e.rs::fake_axhub`.
fn fake_axhub(dir: &Path) -> std::path::PathBuf {
    let axhub = dir.join("axhub");
    std::fs::write(
        &axhub,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.17.3 (commit fake, built fake, fake)"
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ] && [ "$3" = "--json" ]; then
  echo '{"user_email":"ac13@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["read","deploy"]}'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&axhub).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&axhub, perms).unwrap();
    axhub
}

/// Run `prompt-route` with the hook JSON envelope on stdin, anchored at `cwd`
/// (so `routing::find_marker` resolves against a controlled tree) and writing
/// audit lines into `state`'s XDG_STATE_HOME. `AXHUB_NO_AUDIT` is explicitly
/// removed: the whole test depends on audit lines being written.
fn invoke_prompt_route(prompt: &str, cwd: &Path, axhub: &Path, state: &Path) {
    let input =
        serde_json::json!({"hook_event_name":"UserPromptSubmit","prompt":prompt}).to_string();
    let mut command = Command::new(bin());
    command
        .arg("prompt-route")
        .current_dir(cwd)
        .env("AXHUB_BIN", axhub)
        .env("XDG_STATE_HOME", state)
        .env_remove("AXHUB_NO_AUDIT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "prompt-route must fail-open exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_routing_stats(args: &[&str], state: &Path) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .env("XDG_STATE_HOME", state)
        .env_remove("AXHUB_NO_AUDIT")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.spawn().unwrap().wait_with_output().unwrap()
}

/// Read every decision label persisted under `state`'s audit dir.
fn audit_decisions(state: &Path) -> Vec<String> {
    let dir = state.join("axhub-plugin");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("routing-audit-") || !name.ends_with(".jsonl") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(d) = v.get("decision").and_then(|d| d.as_str()) {
                        out.push(d.to_string());
                    }
                }
            }
        }
    }
    out
}

#[test]
fn cli_routing_stats_reports_non_axhub_ignore_rate_ac13() {
    let temp = tempfile::tempdir().unwrap();
    let axhub = fake_axhub(temp.path());
    let state = temp.path().join("state");

    // Non-marker repo: a git root with NO axhub.yaml. find_marker() walking up
    // from here stops at `.git` on the start dir → MarkerStatus::Absent, so a
    // bare-NL prompt routes to Ignore (rule e) regardless of auth.
    let non_marker = temp.path().join("non_marker_repo");
    std::fs::create_dir_all(non_marker.join(".git")).unwrap();

    // Two bare-NL deploy prompts from the non-marker repo → two `ignore` lines.
    invoke_prompt_route("배포해", &non_marker, &axhub, &state);
    invoke_prompt_route("배포해", &non_marker, &axhub, &state);
    // One explicit "axhub" keyword prompt (rule b, marker-independent) → one
    // `axhub` line, so ignore_rate is a real fraction (2/3), not 100%.
    invoke_prompt_route("axhub 로 배포해", &non_marker, &axhub, &state);

    // De-risk a vacuous pass: confirm cwd control actually produced `ignore`
    // lines before trusting the stats math. If current_dir failed, these would
    // be `axhub` (marker Present) and the rate assertion would be meaningless.
    let decisions = audit_decisions(&state);
    let ignore_lines = decisions.iter().filter(|d| *d == "ignore").count();
    let axhub_lines = decisions.iter().filter(|d| *d == "axhub").count();
    assert_eq!(
        ignore_lines, 2,
        "expected 2 non-axhub ignore audit lines, got decisions={decisions:?}"
    );
    assert_eq!(
        axhub_lines, 1,
        "expected 1 axhub audit line, got decisions={decisions:?}"
    );

    // routing-stats --json: the skill-facing report consumes this.
    let stats = run_routing_stats(&["routing-stats", "--json"], &state);
    assert_eq!(
        stats.status.code(),
        Some(0),
        "routing-stats stderr={}",
        String::from_utf8_lossy(&stats.stderr)
    );
    let stdout = String::from_utf8_lossy(&stats.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("routing-stats --json must emit valid JSON");

    assert_eq!(
        parsed["decision_counts"]["ignore"],
        serde_json::json!(2),
        "ignore bucket: {stdout}"
    );
    assert_eq!(
        parsed["decision_counts"]["axhub"],
        serde_json::json!(1),
        "axhub bucket: {stdout}"
    );
    assert_eq!(
        parsed["ignore_count"],
        serde_json::json!(2),
        "ignore_count: {stdout}"
    );
    // ignore_rate = ignore / decided_total = 2 / 3 (legacy lines excluded; here
    // there are none). The non-axhub pass-through signal this AC reports.
    let ignore_rate = parsed["ignore_rate"]
        .as_f64()
        .expect("ignore_rate must be a number");
    assert!(
        (ignore_rate - 2.0 / 3.0).abs() < 1e-9,
        "ignore_rate should be 2/3, got {ignore_rate}: {stdout}"
    );

    // Human-readable report also surfaces the non-axhub ignore rate line.
    let plain = run_routing_stats(&["routing-stats"], &state);
    assert_eq!(plain.status.code(), Some(0));
    let plain_out = String::from_utf8_lossy(&plain.stdout);
    assert!(
        plain_out.contains("ignore 율"),
        "plain report must surface the non-axhub ignore rate: {plain_out}"
    );
}

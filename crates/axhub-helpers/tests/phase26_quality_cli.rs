use std::io::Write;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run_stdin(args: &[&str], stdin: &str, cwd: &std::path::Path, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("AXHUB_DISABLE_HOOKS");
    command.env_remove("AXHUB_DISABLE_HOOK");
    command.env_remove("DISABLE_AXHUB");
    command.env_remove("AXHUB_SKIP_REVIEW");
    command.env_remove("AXHUB_DISABLE_TRIGGERS");
    command.env("AXHUB_NO_AUDIT", "1");
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn json(out: &Output) -> serde_json::Value {
    let raw = stdout(out);
    serde_json::from_str(raw.trim()).unwrap_or_else(|err| panic!("invalid JSON: {err}; raw={raw}"))
}

fn git(cwd: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init"]);
    git(dir.path(), &["config", "user.email", "phase26@example.com"]);
    git(dir.path(), &["config", "user.name", "Phase 26"]);
    std::fs::write(dir.path().join("src.txt"), "hello\n").unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-m", "initial"]);
    dir
}

#[test]
fn commit_gate_asks_for_review_when_commit_or_push_is_unreviewed() {
    let repo = repo();
    let payload = r#"{"tool_name":"Bash","tool_input":{"command":"git commit -am change"}}"#;
    let out = run_stdin(&["commit-gate"], payload, repo.path(), &[]);
    assert!(out.status.success());
    let v = json(&out);
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "ask");
    assert!(v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .contains("review"));
}

#[test]
fn review_acknowledged_then_post_commit_promote_marks_head_reviewed() {
    let repo = repo();
    std::fs::write(repo.path().join("src.txt"), "hello\nreviewed\n").unwrap();

    let review = run_stdin(
        &["state-update", "--review-acknowledged"],
        "",
        repo.path(),
        &[],
    );
    assert!(
        review.status.success(),
        "{}",
        String::from_utf8_lossy(&review.stderr)
    );

    let show = run_stdin(&["state-show", "--json"], "", repo.path(), &[]);
    let state = json(&show);
    assert_eq!(state["review_acknowledged"], true);
    assert!(state["last_reviewed_tree_hash"].as_str().unwrap().len() >= 32);
    assert!(state["last_reviewed_diff_hash"].as_str().unwrap().len() >= 32);

    git(repo.path(), &["add", "src.txt"]);
    git(repo.path(), &["commit", "-m", "reviewed change"]);
    let promote = run_stdin(
        &["state-update", "--post-commit-promote"],
        "",
        repo.path(),
        &[],
    );
    assert!(promote.status.success());

    let show = run_stdin(&["state-show", "--json"], "", repo.path(), &[]);
    let state = json(&show);
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert_eq!(
        state["review_commit_sha"].as_str().unwrap(),
        String::from_utf8_lossy(&head.stdout).trim()
    );
}

#[test]
fn test_classifier_records_failed_test_commands_only() {
    let repo = repo();
    let fail_payload = r#"{"tool_name":"Bash","tool_input":{"command":"bun test"},"tool_response":{"exit_code":1,"stdout":"fail"}}"#;
    let out = run_stdin(&["test-classifier"], fail_payload, repo.path(), &[]);
    assert!(out.status.success());
    let state = json(&run_stdin(&["state-show", "--json"], "", repo.path(), &[]));
    assert!(state["last_test_failure_at"].as_str().is_some());

    let before = state["last_test_failure_at"].as_str().unwrap().to_string();
    let non_test = r#"{"tool_name":"Bash","tool_input":{"command":"ls -la"},"tool_response":{"exit_code":1,"stdout":"fail"}}"#;
    let out = run_stdin(&["test-classifier"], non_test, repo.path(), &[]);
    assert!(out.status.success());
    let state = json(&run_stdin(&["state-show", "--json"], "", repo.path(), &[]));
    assert_eq!(state["last_test_failure_at"].as_str().unwrap(), before);
}

#[test]
fn test_classifier_records_post_tool_use_failure_without_exit_code() {
    let repo = repo();
    let failure_payload = r#"{"hook_event_name":"PostToolUseFailure","tool_name":"Bash","tool_input":{"command":"cargo test"}}"#;
    let out = run_stdin(&["test-classifier"], failure_payload, repo.path(), &[]);
    assert!(out.status.success());
    let state = json(&run_stdin(&["state-show", "--json"], "", repo.path(), &[]));
    assert!(state["last_test_failure_at"].as_str().is_some());
}

#[test]
fn state_update_edit_event_counts_tracked_and_untracked_changes() {
    let repo = repo();
    std::fs::create_dir_all(repo.path().join("src")).unwrap();
    std::fs::create_dir_all(repo.path().join("tests")).unwrap();
    std::fs::write(repo.path().join("src.txt"), "hello\ntracked\n").unwrap();
    std::fs::write(repo.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        repo.path().join("tests/new.test.ts"),
        "test('x', () => {});\n",
    )
    .unwrap();

    let out = run_stdin(&["state-update", "--edit-event"], "", repo.path(), &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let state = json(&run_stdin(&["state-show", "--json"], "", repo.path(), &[]));
    assert_eq!(state["lines_since_review_user"], 3);
    assert_eq!(state["files_changed_since_review"], 3);
    assert_eq!(state["source_files_count"], 1);
    assert_eq!(state["test_files_count"], 1);
    assert_eq!(state["review_acknowledged"], false);
}

#[test]
fn state_update_pull_marks_timestamp_without_edit_counters() {
    let repo = repo();
    std::fs::write(repo.path().join("src.txt"), "hello\npulled\n").unwrap();
    git(repo.path(), &["add", "src.txt"]);
    git(repo.path(), &["commit", "-m", "simulate pull result"]);

    let out = run_stdin(&["state-update", "--pull"], "", repo.path(), &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let state = json(&run_stdin(&["state-show", "--json"], "", repo.path(), &[]));
    assert!(state["last_pull_at"].as_str().is_some());
    assert_eq!(state["lines_since_review_user"], 0);
    assert_eq!(state["files_changed_since_review"], 0);
}

#[test]
fn tdd_inject_emits_option_a_template_for_source_writes_only() {
    let repo = repo();
    let src = r#"{"tool_name":"Edit","tool_input":{"file_path":"src/main.ts"}}"#;
    let out = run_stdin(&["tdd-inject"], src, repo.path(), &[]);
    assert!(out.status.success());
    let v = json(&out);
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("<axhub-tdd-cycle>"));
    assert!(ctx.contains("Observed:"));
    assert!(ctx.contains("Suggested:"));
    assert!(ctx.contains("Skip: AXHUB_DISABLE_HOOK=tdd-inject"));

    let test = r#"{"tool_name":"Edit","tool_input":{"file_path":"src/main.test.ts"}}"#;
    let out = run_stdin(&["tdd-inject"], test, repo.path(), &[]);
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), "{}");
}

#[test]
fn prompt_route_uses_english_additional_context_template() {
    let repo = repo();
    let out = run_stdin(
        &["prompt-route"],
        r#"{"hook_event_name":"UserPromptSubmit","prompt":"오늘 날씨 알려줘"}"#,
        repo.path(),
        &[("AXHUB_BIN", "definitely-missing-axhub")],
    );
    assert!(out.status.success());
    let v = json(&out);
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("<axhub-preflight-status>"));
    assert!(ctx.contains("Observed:"));
    assert!(ctx.contains("Suggested:"));
    assert!(ctx.contains("Skip: AXHUB_DISABLE_HOOK=prompt-route"));
    assert!(!ctx.contains("axhub 버전 확인 결과"));
}

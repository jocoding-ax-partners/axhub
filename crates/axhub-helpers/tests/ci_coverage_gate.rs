use std::collections::HashMap;

use axhub_helpers::commit_gate::{evaluate_bash_command, GateDecision};
use axhub_helpers::consent::decision::{check_decision, issue_decision_token, DecisionVariant};
use axhub_helpers::consent::jwt::ConsentBinding;
use axhub_helpers::hook_output::{
    post_tool_use_context, pre_tool_use_allow, pre_tool_use_ask, pre_tool_use_context,
    pre_tool_use_deny, session_start_context, user_prompt_context, user_prompt_context_with_system,
};
use axhub_helpers::hook_safety::append_hook_error;
use axhub_helpers::karpathy_inject::{user_prompt_karpathy_inject, MAX_KARPATHY_CHARS};
use axhub_helpers::observability::append_autowire_event;
use axhub_helpers::quality_state::{git_stdout, git_tree_hash, QualityState};
use axhub_helpers::settings_merge::MergeOutcome;

struct EnvGuard {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }

    fn remove(key: &'static str) -> Self {
        let old = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.old.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn json(value: &str) -> serde_json::Value {
    serde_json::from_str(value).expect("hook output should be valid json")
}

fn binding() -> ConsentBinding {
    ConsentBinding {
        tool_call_id: "test-tc-coverage".into(),
        action: "diagnose".into(),
        app_id: "axhub".into(),
        profile: "default".into(),
        branch: "main".into(),
        commit_sha: "abc1234".into(),
        context: HashMap::new(),
        synthesized_by_helper: false,
    }
}

#[test]
fn hook_output_helpers_emit_all_permission_and_context_shapes() {
    let session = json(&session_start_context("session ctx"));
    assert_eq!(
        session["hookSpecificOutput"]["hookEventName"],
        "SessionStart"
    );
    assert_eq!(
        session["hookSpecificOutput"]["additionalContext"],
        "session ctx"
    );

    let user = json(&user_prompt_context("user ctx"));
    assert_eq!(
        user["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    assert_eq!(user["hookSpecificOutput"]["additionalContext"], "user ctx");
    assert_eq!(
        user_prompt_context_with_system("ctx", None),
        user_prompt_context("ctx")
    );
    let user_with_system = json(&user_prompt_context_with_system("ctx", Some("visible")));
    assert_eq!(user_with_system["systemMessage"], "visible");

    let pre = json(&pre_tool_use_context("pre ctx"));
    assert_eq!(pre["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(pre["hookSpecificOutput"]["additionalContext"], "pre ctx");
    let post = json(&post_tool_use_context("post ctx"));
    assert_eq!(post["hookSpecificOutput"]["hookEventName"], "PostToolUse");

    let ask = json(&pre_tool_use_ask("needs review"));
    assert_eq!(ask["hookSpecificOutput"]["permissionDecision"], "ask");
    assert_eq!(
        ask["hookSpecificOutput"]["permissionDecisionReason"],
        "needs review"
    );
    let deny = json(&pre_tool_use_deny("unsafe"));
    assert_eq!(deny["hookSpecificOutput"]["permissionDecision"], "deny");
    let allow = json(&pre_tool_use_allow());
    assert_eq!(allow["hookSpecificOutput"]["permissionDecision"], "allow");
}

#[test]
fn karpathy_injection_respects_disable_missing_and_cap_paths() {
    let _lock = axhub_helpers::PROCESS_ENV_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());

    let _disable = EnvGuard::set("AXHUB_DISABLE_KARPATHY", "1");
    assert!(user_prompt_karpathy_inject().unwrap().is_none());
    drop(_disable);

    let missing_root = tempfile::tempdir().unwrap();
    let _root = EnvGuard::set("CLAUDE_PLUGIN_ROOT", missing_root.path());
    let _disable = EnvGuard::remove("AXHUB_DISABLE_KARPATHY");
    let _disable_all = EnvGuard::remove("AXHUB_DISABLE_TRIGGERS");
    assert!(user_prompt_karpathy_inject().unwrap().is_none());
    drop(_root);

    let root = tempfile::tempdir().unwrap();
    let skills_dir = root.path().join("skills/karpathy-guidelines");
    std::fs::create_dir_all(&skills_dir).unwrap();
    let content = "가".repeat(MAX_KARPATHY_CHARS + 5);
    std::fs::write(skills_dir.join("SKILL.md"), &content).unwrap();
    std::fs::write(
        skills_dir.join("SKILL.md.sha256"),
        axhub_helpers::quality_state::sha256_hex(content.as_bytes()),
    )
    .unwrap();
    let _root = EnvGuard::set("CLAUDE_PLUGIN_ROOT", root.path());

    let injected = user_prompt_karpathy_inject().unwrap().unwrap();
    assert_eq!(injected.chars().count(), MAX_KARPATHY_CHARS);
    assert!(content.starts_with(&injected));

    std::fs::write(skills_dir.join("SKILL.md.sha256"), "not-the-current-hash").unwrap();
    let drifted = user_prompt_karpathy_inject().unwrap().unwrap();
    assert_eq!(drifted.chars().count(), MAX_KARPATHY_CHARS);
}

#[test]
fn consent_decision_mints_pending_and_session_tokens() {
    let _lock = axhub_helpers::PROCESS_ENV_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let _state = EnvGuard::set("XDG_STATE_HOME", dir.path().join("state"));
    let _runtime = EnvGuard::set("XDG_RUNTIME_DIR", dir.path().join("runtime"));

    let _session = EnvGuard::remove("CLAUDE_SESSION_ID");
    let pending = issue_decision_token(DecisionVariant::Once, binding())
        .expect("once can mint without session")
        .expect("once returns a token");
    assert!(pending.file_path.contains("consent-pending-"));
    assert!(std::path::Path::new(&pending.file_path).exists());
    drop(_session);

    let _session = EnvGuard::set("CLAUDE_SESSION_ID", "session-decision-test");
    let b = binding();
    let minted = issue_decision_token(DecisionVariant::AllowSession, b.clone())
        .expect("session decision mints")
        .expect("allow-session returns a token");
    assert!(minted
        .file_path
        .contains("consent-session-decision-test.json"));
    assert!(std::path::Path::new(&minted.file_path).exists());
    assert!(check_decision(b.clone()).valid);

    let mut mismatched = b;
    mismatched.action = "different-action".into();
    let verified = check_decision(mismatched);
    assert!(!verified.valid);
    assert_eq!(verified.reason.as_deref(), Some("binding_mismatch:action"));
}

#[test]
fn hook_error_append_writes_private_jsonl() {
    let _lock = axhub_helpers::PROCESS_ENV_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let _state = EnvGuard::set("XDG_STATE_HOME", dir.path());

    append_hook_error("ci-coverage", &"boom");

    let path = dir.path().join("axhub-plugin/hook-errors.jsonl");
    let content = std::fs::read_to_string(path).expect("hook error log should be written");
    let line: serde_json::Value =
        serde_json::from_str(content.trim()).expect("hook error line is json");
    assert_eq!(line["hook"], "ci-coverage");
    assert_eq!(line["error"], "boom");
}

#[test]
fn commit_gate_allows_reviewed_and_explicitly_skipped_commands() {
    let _lock = axhub_helpers::PROCESS_ENV_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .expect("git init");
    std::process::Command::new("git")
        .args(["config", "user.email", "ci@example.com"])
        .current_dir(repo)
        .output()
        .expect("git config email");
    std::process::Command::new("git")
        .args(["config", "user.name", "CI"])
        .current_dir(repo)
        .output()
        .expect("git config name");
    std::fs::write(repo.join("file.txt"), "hello").unwrap();
    std::process::Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(repo)
        .output()
        .expect("git add");
    let commit = std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .expect("git commit");
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );

    let state = QualityState::default();
    assert_eq!(
        evaluate_bash_command("echo not git", &state, repo),
        GateDecision::Allow
    );
    assert_eq!(
        evaluate_bash_command(
            "git commit -m nope",
            &state,
            dir.path().join("missing").as_path()
        ),
        GateDecision::Allow
    );

    let _skip = EnvGuard::set("AXHUB_SKIP_REVIEW", "1");
    assert_eq!(
        evaluate_bash_command("git push", &state, repo),
        GateDecision::Allow
    );
    drop(_skip);

    assert!(matches!(
        evaluate_bash_command("git push", &state, repo),
        GateDecision::Ask(reason) if reason.contains("review missing")
    ));

    let head = git_stdout(repo, &["rev-parse", "HEAD"]).unwrap();
    let reviewed_head = QualityState {
        review_commit_sha: Some(head),
        ..QualityState::default()
    };
    assert_eq!(
        evaluate_bash_command("git commit -m ok", &reviewed_head, repo),
        GateDecision::Allow
    );

    let reviewed_tree = QualityState {
        review_acknowledged: true,
        last_reviewed_tree_hash: Some(git_tree_hash(repo).unwrap()),
        ..QualityState::default()
    };
    assert_eq!(
        evaluate_bash_command("git push origin main", &reviewed_tree, repo),
        GateDecision::Allow
    );
}

#[test]
fn autowire_observability_reuses_existing_salt_and_hashes_preserved_commands() {
    let _lock = axhub_helpers::PROCESS_ENV_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let _state = EnvGuard::set("XDG_STATE_HOME", dir.path());
    let state_dir = dir.path().join("axhub-plugin");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join("observability-salt"),
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )
    .unwrap();

    append_autowire_event(
        &MergeOutcome::PreservedOther,
        "project",
        Some("echo secret"),
    )
    .expect("preserved-other event appends");
    append_autowire_event(&MergeOutcome::Created, "user", None).expect("create event appends");
    append_autowire_event(&MergeOutcome::Merged, "project", None).expect("merge event appends");
    append_autowire_event(&MergeOutcome::InvalidJson, "user", None).expect("abort event appends");
    append_autowire_event(&MergeOutcome::PartialSchema, "user", None)
        .expect("partial-schema event appends");
    append_autowire_event(&MergeOutcome::PermissionDenied, "user", None)
        .expect("permission-denied event appends");

    let content = std::fs::read_to_string(state_dir.join("events.jsonl")).unwrap();
    let rows = content
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows[0]["action"], "preserve");
    assert_eq!(rows[0]["branch"], 5);
    assert_eq!(rows[0]["scope"], "project");
    assert!(rows[0]["other_command_hash"]
        .as_str()
        .unwrap()
        .starts_with("hmac-sha256:"));
    assert_eq!(rows[1]["action"], "create");
    assert_eq!(rows[1]["branch"], 1);
    assert_eq!(rows[2]["action"], "merge");
    assert_eq!(rows[2]["branch"], 3);
    assert_eq!(rows[3]["action"], "abort");
    assert_eq!(rows[3]["branch"], 6);
    assert_eq!(rows[4]["branch"], 7);
    assert_eq!(rows[5]["branch"], 8);
}

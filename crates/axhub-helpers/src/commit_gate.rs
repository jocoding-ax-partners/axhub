use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::quality_state::{git_tree_hash, QualityState};

static GIT_COMMIT_OR_PUSH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*git(\.exe)?\s+(commit|push)\b").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    Ask(String),
}

pub fn evaluate_bash_command(
    command: &str,
    state: &QualityState,
    repo_root: &Path,
) -> GateDecision {
    if !GIT_COMMIT_OR_PUSH_RE.is_match(command) {
        return GateDecision::Allow;
    }
    if std::env::var("AXHUB_SKIP_REVIEW").as_deref() == Ok("1")
        || crate::hook_safety::is_quality_triggers_disabled()
    {
        return GateDecision::Allow;
    }
    let head = match crate::quality_state::git_stdout(repo_root, &["rev-parse", "HEAD"]) {
        Ok(head) => head,
        Err(_) => return GateDecision::Allow,
    };
    if state.review_commit_sha.as_deref() == Some(head.as_str()) {
        return GateDecision::Allow;
    }
    if state.review_acknowledged {
        if let (Some(reviewed), Ok(current)) = (
            state.last_reviewed_tree_hash.as_deref(),
            git_tree_hash(repo_root),
        ) {
            if reviewed == current {
                return GateDecision::Allow;
            }
        }
    }
    GateDecision::Ask("review missing or code changed after review. Run axhub-review first?".into())
}

pub fn is_commit_or_push(command: &str) -> bool {
    GIT_COMMIT_OR_PUSH_RE.is_match(command)
}

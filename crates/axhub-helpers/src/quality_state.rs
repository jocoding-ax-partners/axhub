use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::atomic_jsonl;
use crate::runtime_paths::state_dir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QualityState {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub last_review_at: Option<String>,
    #[serde(default)]
    pub review_acknowledged: bool,
    #[serde(default)]
    pub last_reviewed_base_sha: Option<String>,
    #[serde(default)]
    pub last_reviewed_tree_hash: Option<String>,
    #[serde(default)]
    pub last_reviewed_diff_hash: Option<String>,
    #[serde(default)]
    pub review_commit_sha: Option<String>,
    #[serde(default)]
    pub lines_since_review_user: u64,
    #[serde(default)]
    pub files_changed_since_review: u64,
    #[serde(default)]
    pub test_files_count: u64,
    #[serde(default)]
    pub source_files_count: u64,
    #[serde(default)]
    pub corrupt_msg_shown: bool,
    #[serde(default)]
    pub last_test_failure_at: Option<String>,
    #[serde(default)]
    pub last_debug_at: Option<String>,
    #[serde(default)]
    pub last_shipped_at: Option<String>,
    #[serde(default)]
    pub last_pull_at: Option<String>,
}

fn default_version() -> u32 {
    1
}

impl Default for QualityState {
    fn default() -> Self {
        Self {
            version: 1,
            last_review_at: None,
            review_acknowledged: false,
            last_reviewed_base_sha: None,
            last_reviewed_tree_hash: None,
            last_reviewed_diff_hash: None,
            review_commit_sha: None,
            lines_since_review_user: 0,
            files_changed_since_review: 0,
            test_files_count: 0,
            source_files_count: 0,
            corrupt_msg_shown: false,
            last_test_failure_at: None,
            last_debug_at: None,
            last_shipped_at: None,
            last_pull_at: None,
        }
    }
}

impl QualityState {
    pub fn load_or_init(repo_root: &Path) -> Result<Self> {
        let path = state_path(repo_root);
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        match serde_json::from_str::<Self>(&raw) {
            Ok(mut state) => {
                if state.version == 0 {
                    state.version = 1;
                }
                Ok(state)
            }
            Err(err) => {
                corrupt_backup(repo_root, &err.to_string()).ok();
                Ok(Self::default())
            }
        }
    }

    pub fn save_atomic(&self, repo_root: &Path) -> Result<()> {
        let path = state_path(repo_root);
        let parent = path.parent().context("quality state parent")?;
        fs::create_dir_all(parent)?;
        let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        let bytes = serde_json::to_vec_pretty(self)?;
        fs::write(&tmp, bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&tmp)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&tmp, perms)?;
        }
        fs::rename(&tmp, &path)?;
        Ok(())
    }
}

pub fn repo_root_from_cwd() -> Result<PathBuf> {
    if let Some(root) = std::env::var_os("AXHUB_REPO_ROOT").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(root));
    }
    let cwd = std::env::current_dir()?;
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output();
    if let Ok(out) = out {
        if out.status.success() {
            let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !root.is_empty() {
                return Ok(PathBuf::from(root));
            }
        }
    }
    Ok(cwd)
}

pub fn state_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".axhub-state").join("quality.json")
}

pub fn state_show_json(repo_root: &Path) -> Result<String> {
    Ok(serde_json::to_string_pretty(&QualityState::load_or_init(
        repo_root,
    )?)?)
}

pub fn update_review_acknowledged(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    state.last_review_at = Some(now());
    state.review_acknowledged = true;
    state.review_commit_sha = None;
    state.last_reviewed_base_sha = git_stdout(repo_root, &["rev-parse", "HEAD"]).ok();
    state.last_reviewed_tree_hash = git_tree_hash(repo_root).ok();
    state.last_reviewed_diff_hash = git_diff_hash(repo_root, &["diff", "HEAD"]).ok();
    state.lines_since_review_user = 0;
    state.files_changed_since_review = 0;
    state.save_atomic(repo_root)
}

pub fn update_post_commit_promote(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    if !state.review_acknowledged {
        return Ok(());
    }
    let head = match git_stdout(repo_root, &["rev-parse", "HEAD"]) {
        Ok(head) => head,
        Err(_) => return Ok(()),
    };
    let committed_tree_hash = git_tree_hash(repo_root).ok();
    let committed_diff_hash = state
        .last_reviewed_base_sha
        .as_ref()
        .and_then(|base| git_diff_hash(repo_root, &["diff", &format!("{base}..HEAD")]).ok());

    let tree_match =
        committed_tree_hash.is_some() && committed_tree_hash == state.last_reviewed_tree_hash;
    let diff_match =
        committed_diff_hash.is_some() && committed_diff_hash == state.last_reviewed_diff_hash;
    if tree_match || diff_match {
        state.review_commit_sha = Some(head);
    } else {
        state.review_acknowledged = false;
        state.review_commit_sha = None;
    }
    state.save_atomic(repo_root)
}

pub fn update_edit_event(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    let (lines, files, source_files, test_files) = current_worktree_numstat(repo_root)?;
    state.lines_since_review_user = lines;
    state.files_changed_since_review = files;
    state.source_files_count = source_files;
    state.test_files_count = test_files;
    state.review_acknowledged = false;
    state.review_commit_sha = None;
    state.save_atomic(repo_root)
}

pub fn mark_test_failure(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    state.last_test_failure_at = Some(now());
    state.save_atomic(repo_root)
}

pub fn mark_debug_acknowledged(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    state.last_debug_at = Some(now());
    state.save_atomic(repo_root)
}

pub fn mark_shipped(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    state.last_shipped_at = Some(now());
    state.save_atomic(repo_root)
}

pub fn mark_pull(repo_root: &Path) -> Result<()> {
    let mut state = QualityState::load_or_init(repo_root)?;
    state.last_pull_at = Some(now());
    state.save_atomic(repo_root)
}

pub fn corrupt_backup(repo_root: &Path, error: &str) -> Result<()> {
    let path = state_path(repo_root);
    if path.exists() {
        let backup =
            path.with_file_name(format!("quality.json.corrupt-{}", Utc::now().timestamp()));
        let _ = fs::rename(&path, &backup);
        if let Some(dir) = state_dir() {
            let log = dir.join("state-corrupt.jsonl");
            let line = serde_json::json!({
                "ts": now(),
                "repo": repo_root.display().to_string(),
                "backup": backup.display().to_string(),
                "error": error,
            })
            .to_string();
            atomic_jsonl::append_line(&log, &line).ok();
        }
    }
    Ok(())
}

pub fn migrate(value: Value, _from: u32, _to: u32) -> Result<QualityState> {
    Ok(serde_json::from_value(value).unwrap_or_default())
}

fn current_worktree_numstat(repo_root: &Path) -> Result<(u64, u64, u64, u64)> {
    let out = git_stdout(repo_root, &["diff", "HEAD", "--numstat"])?;
    let mut lines = 0_u64;
    let mut files = 0_u64;
    let mut source_files = 0_u64;
    let mut test_files = 0_u64;
    for row in out.lines() {
        let parts: Vec<_> = row.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let path = parts[2];
        if excluded_path(path) || parts[0] == "-" || parts[1] == "-" {
            continue;
        }
        let added = parts[0].parse::<u64>().unwrap_or(0);
        let deleted = parts[1].parse::<u64>().unwrap_or(0);
        lines += added + deleted;
        files += 1;
        if is_test_path(path) {
            test_files += 1;
        } else if is_source_path(path) {
            source_files += 1;
        }
    }
    let untracked = git_stdout(repo_root, &["ls-files", "--others", "--exclude-standard"])?;
    for path in untracked.lines() {
        if path.is_empty() || excluded_path(path) {
            continue;
        }
        let file_path = repo_root.join(path);
        let Ok(content) = fs::read_to_string(&file_path) else {
            continue;
        };
        lines += content.lines().count() as u64;
        files += 1;
        if is_test_path(path) {
            test_files += 1;
        } else if is_source_path(path) {
            source_files += 1;
        }
    }
    Ok((lines, files, source_files, test_files))
}

fn excluded_path(path: &str) -> bool {
    path.starts_with("vendor/")
        || path.starts_with("node_modules/")
        || path.starts_with("target/")
        || path.starts_with(".git/")
}

fn is_test_path(path: &str) -> bool {
    has_test_segment(path)
        || path.contains(".test.")
        || path.contains(".spec.")
        || path.ends_with("_test.go")
        || path.ends_with("_test.rs")
}

fn has_test_segment(path: &str) -> bool {
    for segment in ["test", "tests", "__tests__"] {
        let prefix = format!("{segment}/");
        let infix = format!("/{segment}/");
        if path == segment || path.starts_with(&prefix) || path.contains(&infix) {
            return true;
        }
    }
    false
}

fn is_source_path(path: &str) -> bool {
    matches!(
        Path::new(path).extension().and_then(|e| e.to_str()),
        Some(
            "ts" | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "rs"
                | "go"
                | "java"
                | "rb"
                | "swift"
                | "kt"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "ipynb"
        )
    )
}

pub fn git_tree_hash(repo_root: &Path) -> Result<String> {
    let tree = git_stdout(repo_root, &["ls-tree", "-r", "HEAD"])?;
    Ok(sha256_hex(tree.as_bytes()))
}

pub fn git_diff_hash(repo_root: &Path, args: &[&str]) -> Result<String> {
    let diff = git_stdout(repo_root, args)?;
    Ok(sha256_hex(diff.as_bytes()))
}

pub fn git_stdout(repo_root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()?;
    if !out.status.success() {
        anyhow::bail!("git {:?} failed", args);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

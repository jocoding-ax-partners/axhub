//! Phase 5P learning emit — plan v6 §4.6.
//!
//! Appends one entry per successful loop to `learnings.jsonl` so the
//! recurrence detector + future server aggregation (v0.9) can correlate
//! recurring error classes.

use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::DiagnoseError;
use crate::atomic_jsonl::{append_line, read_lines};
use crate::consent::key::state_root;

const LEARNINGS_DIR: &str = "learnings";
const LEARNINGS_FILE: &str = "learnings.jsonl";

/// Enum form of "fail" / "pass" so a typo cannot land in
/// `loop_signal_before` / `loop_signal_after` and silently break recurrence
/// aggregation. Serialised as snake_case so existing JSONL files
/// (`"fail"` / `"pass"`) deserialise without migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopSignalState {
    Fail,
    Pass,
}

impl LoopSignalState {
    pub fn as_str(self) -> &'static str {
        match self {
            LoopSignalState::Fail => "fail",
            LoopSignalState::Pass => "pass",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningEntry {
    pub ts: String,
    pub error_class: String,
    pub winning_hypothesis: String,
    pub fix_action: String,
    pub loop_signal_before: LoopSignalState,
    pub loop_signal_after: LoopSignalState,
    pub cwd_hash: String,
    pub loop_id: String,
}

pub fn learnings_path() -> PathBuf {
    state_root().join(LEARNINGS_DIR).join(LEARNINGS_FILE)
}

pub fn hash_cwd(cwd: &std::path::Path) -> String {
    let mut h = Sha256::new();
    h.update(cwd.to_string_lossy().as_bytes());
    format!("sha256:{:x}", h.finalize())
}

pub fn emit(entry: &LearningEntry) -> Result<(), DiagnoseError> {
    emit_to(&learnings_path(), entry)
}

pub fn emit_to(path: &std::path::Path, entry: &LearningEntry) -> Result<(), DiagnoseError> {
    let line = serde_json::to_string(entry)
        .map_err(|e| DiagnoseError::LearningEmitFailed(e.to_string()))?;
    append_line(path, &line).map_err(|e| DiagnoseError::LearningEmitFailed(e.to_string()))
}

pub fn read_all_from(path: &std::path::Path) -> Result<Vec<LearningEntry>, DiagnoseError> {
    let entries = read_lines(path, |line| {
        serde_json::from_str::<LearningEntry>(line).ok()
    })
    .map_err(|e| DiagnoseError::LearningEmitFailed(e.to_string()))?;
    Ok(entries)
}

pub fn builder(error_class: impl Into<String>) -> LearningEntry {
    LearningEntry {
        ts: Utc::now().to_rfc3339(),
        error_class: error_class.into(),
        winning_hypothesis: String::new(),
        fix_action: String::new(),
        loop_signal_before: LoopSignalState::Fail,
        loop_signal_after: LoopSignalState::Pass,
        cwd_hash: String::new(),
        loop_id: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn emit_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("learnings.jsonl");
        let mut e = builder("npm_eacces");
        e.winning_hypothesis = "permission".into();
        e.fix_action = "chown".into();
        e.cwd_hash = hash_cwd(std::path::Path::new("/proj/x"));
        e.loop_id = "L-emit-1".into();
        emit_to(&path, &e).unwrap();
        let all = read_all_from(&path).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], e);
    }

    #[test]
    fn hash_cwd_is_stable() {
        let h1 = hash_cwd(std::path::Path::new("/proj/x"));
        let h2 = hash_cwd(std::path::Path::new("/proj/x"));
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn distinct_paths_yield_distinct_hashes() {
        let a = hash_cwd(std::path::Path::new("/proj/x"));
        let b = hash_cwd(std::path::Path::new("/proj/y"));
        assert_ne!(a, b);
    }

    #[test]
    fn loop_signal_state_as_str_matches_serialized_contract() {
        assert_eq!(LoopSignalState::Fail.as_str(), "fail");
        assert_eq!(LoopSignalState::Pass.as_str(), "pass");
        assert_eq!(
            serde_json::to_string(&LoopSignalState::Fail).unwrap(),
            "\"fail\""
        );
        assert_eq!(
            serde_json::from_str::<LoopSignalState>("\"pass\"").unwrap(),
            LoopSignalState::Pass
        );
    }

    #[test]
    fn read_all_skips_corrupt_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("learnings.jsonl");
        let mut e = builder("network_timeout");
        e.loop_id = "L-corrupt".into();
        std::fs::write(
            &path,
            format!(
                "{}\nnot-json\n",
                serde_json::to_string(&e).expect("entry serializes")
            ),
        )
        .unwrap();

        let all = read_all_from(&path).unwrap();
        assert_eq!(all, vec![e]);
    }
}

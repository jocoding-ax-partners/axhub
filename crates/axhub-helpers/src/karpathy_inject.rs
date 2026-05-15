use std::path::PathBuf;

use anyhow::Result;

use crate::quality_state::sha256_hex;

pub const MAX_KARPATHY_CHARS: usize = 10_000;

pub fn user_prompt_karpathy_inject() -> Result<Option<String>> {
    if std::env::var("AXHUB_DISABLE_KARPATHY").as_deref() == Ok("1")
        || std::env::var("AXHUB_DISABLE_TRIGGERS").as_deref() == Ok("1")
    {
        return Ok(None);
    }
    let root = plugin_root();
    let path = root.join("skills/karpathy-guidelines/SKILL.md");
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[axhub] karpathy-guidelines missing: {err}");
            return Ok(None);
        }
    };
    let hash = sha256_hex(content.as_bytes());
    let expected_path = root.join("skills/karpathy-guidelines/SKILL.md.sha256");
    if let Ok(expected) = std::fs::read_to_string(expected_path) {
        if hash != expected.trim() {
            eprintln!("[axhub] karpathy-guidelines drift detected. release rebuild needed.");
        }
    }
    let capped: String = content.chars().take(MAX_KARPATHY_CHARS).collect();
    if capped.len() < content.len() {
        eprintln!("[axhub] karpathy-guidelines >10K chars. truncating.");
    }
    Ok(Some(capped))
}

fn plugin_root() -> PathBuf {
    std::env::var_os("CLAUDE_PLUGIN_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

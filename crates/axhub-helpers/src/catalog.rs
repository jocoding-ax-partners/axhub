use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/catalog_generated.rs"));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorEntry {
    pub emotion: String,
    pub cause: String,
    pub action: String,
    pub button: Option<String>,
}

static CATALOG: LazyLock<BTreeMap<String, ErrorEntry>> =
    LazyLock::new(|| serde_json::from_str(CATALOG_JSON).expect("generated catalog JSON is valid"));

fn default_entry() -> ErrorEntry {
    ErrorEntry {
        emotion: "이건 흔한 일이에요.".to_string(),
        cause: "알 수 없는 에러가 생겼어요.".to_string(),
        action: "관리자에게 물어봐주세요.".to_string(),
        button: None,
    }
}

pub fn catalog_len() -> usize {
    CATALOG.len()
}

pub fn classify(exit_code: i32, stdout: &str) -> ErrorEntry {
    let error_code = serde_json::from_str::<serde_json::Value>(stdout)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str())
                .map(ToOwned::to_owned)
        });
    if let Some(code) = error_code {
        let sub_key = format!("{exit_code}:{code}");
        if let Some(entry) = CATALOG.get(&sub_key) {
            return entry.clone();
        }
    }
    CATALOG
        .get(&exit_code.to_string())
        .cloned()
        .unwrap_or_else(default_entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_catalog_has_expected_entries() {
        assert!(catalog_len() >= 13);
        assert!(classify(0, "").emotion.contains("축하해요"));
        let sub = classify(64, r#"{"error":{"code":"validation.app_ambiguous"}}"#);
        assert!(sub.emotion.contains("같은 이름이 두 개"));
        assert!(classify(99, "not-json").cause.contains("알 수 없는 에러"));
    }
}

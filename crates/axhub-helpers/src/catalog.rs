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
    // The spawned `axhub` CLI emits a closed-enum `error.code` (auth, not_found,
    // ...) plus an optional dotted `error.subcode` (update.downgrade_blocked,
    // fleet_partial_failure, ...) carrying finer detail. Resolve most-specific
    // first: `{exit}:{subcode}` -> `{exit}:{code}` -> `{exit}` -> default.
    let envelope = serde_json::from_str::<serde_json::Value>(stdout).ok();
    let field = |name: &str| -> Option<String> {
        envelope.as_ref().and_then(|v| {
            v.get("error")
                .and_then(|e| e.get(name))
                .and_then(|c| c.as_str())
                .map(ToOwned::to_owned)
        })
    };
    let subcode = field("subcode");
    let code = field("code");
    for detail in [subcode.as_deref(), code.as_deref()].into_iter().flatten() {
        if let Some(entry) = CATALOG.get(&format!("{exit_code}:{detail}")) {
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
        // Real envelope shape: dotted detail rides `error.subcode`, the closed
        // enum rides `error.code`. The fine `{exit}:{subcode}` key wins.
        let sub = classify(
            64,
            r#"{"error":{"code":"usage","subcode":"validation.app_ambiguous"}}"#,
        );
        assert!(sub.emotion.contains("같은 이름이 두 개"));
        assert!(classify(99, "not-json").cause.contains("알 수 없는 에러"));
    }

    #[test]
    fn classify_prefers_subcode_tier_over_code_and_exit() {
        // exit 66 + subcode update.cosign_enforce_failed must hit the most
        // specific fine entry, not the generic base-66 safety-block copy.
        let entry = classify(
            66,
            r#"{"error":{"code":"other","subcode":"update.cosign_enforce_failed"}}"#,
        );
        assert!(entry.action.contains("IT 보안 담당자"));
        assert_ne!(entry, classify(66, ""));
    }

    #[test]
    fn classify_falls_to_code_tier_when_no_subcode_key() {
        // exit 65 + code apis.call_consent_required has only a `{exit}:{code}`
        // entry (no subcode key) — the code tier resolves it.
        let entry = classify(65, r#"{"error":{"code":"apis.call_consent_required"}}"#);
        assert!(entry.cause.contains("서버 상태"));
    }

    #[test]
    fn classify_falls_to_exit_base_when_no_fine_key() {
        // exit 4 (current CLI unauth) with an enum code that has no fine key
        // resolves to the neutral base-4 auth entry, never the default.
        let entry = classify(4, r#"{"error":{"code":"auth"}}"#);
        assert!(!entry.cause.contains("알 수 없는 에러"));
        assert!(entry.emotion.contains("로그인"));
        assert_eq!(entry, classify(4, ""));
    }

    #[test]
    fn classify_real_cli_exit_codes_never_default() {
        // The live CLI emits 0/1/2/4..15/64/66 — none may fall through to the
        // unknown-error default. (65/67/68 are the helper's own output namespace
        // and are also covered.)
        for exit in [
            0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 64, 65, 66, 67, 68,
        ] {
            let entry = classify(exit, "");
            assert!(
                !entry.cause.contains("알 수 없는 에러"),
                "exit {exit} must have a dedicated base entry, got default"
            );
        }
    }

    #[test]
    fn classify_unknown_exit_with_unmatched_detail_defaults() {
        // An exit with no base entry and an envelope whose code/subcode match no
        // fine key must fall to the default — the negative path stays reachable.
        let entry = classify(
            255,
            r#"{"error":{"code":"auth","subcode":"nope.nonexistent"}}"#,
        );
        assert!(entry.cause.contains("알 수 없는 에러"));
    }
}

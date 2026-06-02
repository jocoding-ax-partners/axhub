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

/// Bridge the two frozen failure-exit spaces into the catalog's CLI-native keys.
///
/// spec 004 surfaced that `classify-exit` is fed from BOTH frozen contracts:
/// - the current ax-hub-cli emits native codes (auth=4, not_found=5,
///   rate_limited=6, api/internal=7) — external, not editable here;
/// - the axhub-helpers binaries (`preflight`, `quality_gate`, `bootstrap`,
///   `deploy_prep`, `list_deployments`, `token-gate`, `sync`) keep a
///   sysexits-derived OUTPUT contract (EXIT_AUTH=65, EXIT_NOT_FOUND=67,
///   rate=68, internal=70) that is a TESTED PUBLIC surface and MUST NOT change.
///
/// The catalog is keyed on the CLI-native space, so a helper exit is normalized
/// into it before lookup. This mirrors `list_deployments::exit_to_error_code`
/// (`4 | 65 => auth`, `5 | 67 => not_found`), the existing dual-mapping
/// precedent. 64 (usage) and 66 (scope/update) are shared across both spaces
/// and pass through unchanged.
pub(crate) fn normalize_helper_exit(exit_code: i32) -> i32 {
    match exit_code {
        65 => 4, // helper EXIT_AUTH       → CLI auth
        67 => 5, // helper EXIT_NOT_FOUND  → CLI not_found
        68 => 6, // helper rate-limit      → CLI rate_limited
        70 => 7, // helper internal        → CLI api/internal
        other => other,
    }
}

pub fn classify(exit_code: i32, stdout: &str) -> ErrorEntry {
    // Normalize helper-output exits (65/67/68/70) into the CLI-native space the
    // catalog is keyed on, so both frozen contracts route to one template.
    let exit_code = normalize_helper_exit(exit_code);
    // spec 004 S3: the current CLI puts a coarse slug in `error.code` (e.g.
    // "usage", "other") and the fine-grained discriminator in `error.subcode`
    // (e.g. "update.cosign_enforce_failed"). Match the most specific key first:
    // {exit}:{subcode} → {exit}:{code} → base {exit}. Falling through `code`
    // preserves legacy callers/tests that put the fine value in `error.code`.
    let parsed = serde_json::from_str::<serde_json::Value>(stdout).ok();
    let field = |name: &str| {
        parsed
            .as_ref()
            .and_then(|v| v.get("error"))
            .and_then(|e| e.get(name))
            .and_then(|c| c.as_str())
            .map(ToOwned::to_owned)
    };
    for key in [field("subcode"), field("code")].into_iter().flatten() {
        if let Some(entry) = CATALOG.get(&format!("{exit_code}:{key}")) {
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
        // Real CLI shape: fine id in `subcode`, coarse `code` alongside — subcode wins.
        let by_subcode = classify(
            66,
            r#"{"error":{"code":"other","subcode":"update.cosign_enforce_failed"}}"#,
        );
        assert!(by_subcode.action.contains("IT 보안 담당자"));
        let downgrade_subcode = classify(
            66,
            r#"{"error":{"code":"other","subcode":"update.downgrade_blocked"}}"#,
        );
        assert!(downgrade_subcode
            .emotion
            .contains("더 낮은 버전으로 되돌리려는"));
        assert!(classify(99, "not-json").cause.contains("알 수 없는 에러"));
        // US1 (spec 004): CLI auth = exit 4 + error.code "auth" (was sysexits 65).
        assert!(classify(4, r#"{"error":{"code":"auth"}}"#)
            .action
            .contains("다시 로그인"));
        // US2 (spec 004): CLI not_found = exit 5 + error.code "not_found" (was sysexits 67).
        assert!(classify(5, r#"{"error":{"code":"not_found"}}"#)
            .emotion
            .contains("못 찾았어요"));
        // US4 (spec 004): CLI rate_limited = exit 6 (was sysexits 68).
        assert!(classify(6, r#"{"error":{"code":"rate_limited"}}"#)
            .emotion
            .contains("너무 많이"));
        // S3: subcode is matched ahead of coarse code for {exit}:{subcode} keys.
        let sub = classify(
            64,
            r#"{"error":{"code":"usage","subcode":"validation.app_ambiguous"}}"#,
        );
        assert!(sub.emotion.contains("같은 이름이 두 개"));
    }

    #[test]
    fn helper_output_exits_normalize_into_cli_space() {
        // spec 004: the axhub-helpers OUTPUT contract (preflight/deploy_prep/
        // bootstrap/list_deployments) keeps sysexits 65/67/68/70. Those reach
        // classify-exit too and must resolve to the same template as the CLI
        // native codes 4/5/6/7. Both frozen contracts → one empathy template.
        assert_eq!(
            classify(65, r#"{"error":{"code":"auth"}}"#),
            classify(4, r#"{"error":{"code":"auth"}}"#)
        );
        assert_eq!(
            classify(67, r#"{"error":{"code":"not_found"}}"#),
            classify(5, r#"{"error":{"code":"not_found"}}"#)
        );
        assert_eq!(classify(68, ""), classify(6, ""));
        assert_eq!(classify(70, ""), classify(7, ""));
        // 70→7 is non-vacuous: base "7" exists, so this resolves a real
        // internal-error template, NOT default_entry.
        assert!(classify(70, "").emotion.contains("서버 내부"));
        assert!(
            classify(70, r#"{"error":{"code":"catalog.internal_error"}}"#)
                .cause
                .contains("catalog 서버")
        );
        // base fallback (no error.code) still lands on the right template
        assert!(classify(65, "").action.contains("다시 로그인"));
        assert!(classify(67, "").emotion.contains("못 찾았어요"));
        assert!(classify(68, "").emotion.contains("너무 많이"));
        // helper not_found (67) + plugin subcode resolves via the normalized key
        assert!(
            classify(67, r#"{"error":{"code":"github.install_not_found"}}"#)
                .button
                .is_some_and(|button| button.contains("GitHub 연결 링크"))
        );
    }

    #[test]
    fn normalize_matches_pinned_contract_json() {
        // spec 004 M1: cli-exit-contract.json::helper_output_normalization is a
        // doc mirror of normalize_helper_exit. Cross-check them here so the JSON
        // and the Rust fn cannot silently drift apart.
        let contract_path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/cli-exit-contract.json");
        let raw = std::fs::read_to_string(contract_path).expect("read cli-exit-contract.json");
        let json: serde_json::Value = serde_json::from_str(&raw).expect("valid contract JSON");
        let map = json
            .get("helper_output_normalization")
            .and_then(|v| v.as_object())
            .expect("helper_output_normalization object");
        let mut checked = 0;
        for (helper, cli) in map {
            let Ok(helper_code) = helper.parse::<i32>() else {
                continue; // skip the _comment string key
            };
            let cli_code = cli.as_i64().expect("normalization target is an integer") as i32;
            assert_eq!(
                normalize_helper_exit(helper_code),
                cli_code,
                "contract says {helper_code}->{cli_code}, normalize_helper_exit disagrees"
            );
            checked += 1;
        }
        assert_eq!(checked, 4, "expected 65/67/68/70 normalization entries");
    }
}

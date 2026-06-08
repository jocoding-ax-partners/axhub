//! User-app deploy artifact sanity verifier.
//!
//! Boundary: this module verifies the JSON-ish stdout returned by
//! `axhub deploy create --json` for a user's app. It intentionally does not
//! share code with release artifact verification (`scripts/release-check.ts`),
//! which validates axhub helper release binaries. The verifier is advisory and
//! fail-open: absence of parseable object JSON means "no signal", not failure.

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyResult {
    pub passed: bool,
    pub violations: Vec<String>,
}

pub fn verify_user_app_artifact(deploy_stdout: &str) -> VerifyResult {
    let mut violations = Vec::new();
    let Some(response) = parse_deploy_response(deploy_stdout) else {
        return VerifyResult {
            passed: true,
            violations,
        };
    };

    if let Some(manifest_hash) = response.get("manifest_hash") {
        if !manifest_hash.as_str().is_some_and(is_sha256_hex) {
            violations.push("manifest_hash 형식이 sha256 hex (64자 hex) 가 아니에요".to_string());
        }
    }

    if let Some(state) = response.get("state") {
        let normalized = js_string(state).to_lowercase();
        if !matches!(
            normalized.as_str(),
            "live" | "running" | "deployed" | "active" | "ok" | "succeeded" | "success"
        ) {
            violations.push(format!(
                "state=\"{}\" — live/running/deployed 가 아니에요",
                js_string(state)
            ));
        }
    }

    if let Some(url) = response.get("url") {
        let valid = url.as_str().is_some_and(|url| {
            let lower = url.to_lowercase();
            lower.starts_with("http://") || lower.starts_with("https://")
        });
        if !valid {
            violations.push(format!(
                "url=\"{}\" 가 http(s):// 로 시작 안 해요",
                js_string(url)
            ));
        }
    }

    for id_key in ["deployment_id", "deploy_id", "id"] {
        if let Some(value) = response.get(id_key) {
            if value.as_str().is_none_or(|id| id.trim().is_empty()) {
                violations.push(format!("{id_key} 가 비어 있어요"));
            }
            break;
        }
    }

    VerifyResult {
        passed: violations.is_empty(),
        violations,
    }
}

fn parse_deploy_response(stdout: &str) -> Option<Map<String, Value>> {
    let trimmed = stdout.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let parsed: Value = serde_json::from_str(trimmed).ok()?;
    match parsed {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.as_bytes().iter().all(u8::is_ascii_hexdigit)
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

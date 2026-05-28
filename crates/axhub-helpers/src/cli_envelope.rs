use serde_json::Value;

pub fn parse_json_stdout(stdout: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(stdout.trim())
}

pub fn is_envelope(value: &Value) -> bool {
    value.get("schema_version").is_some() || value.get("status").is_some()
}

pub fn unwrap_data(value: &Value) -> &Value {
    if is_envelope(value) {
        value.get("data").unwrap_or(value)
    } else {
        value
    }
}

pub fn envelope_status(value: &Value) -> Option<&str> {
    value.get("status").and_then(Value::as_str)
}

pub fn error_code(value: &Value) -> Option<String> {
    value
        .pointer("/error/subcode")
        .or_else(|| value.pointer("/error/code"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/hint")
        .or_else(|| value.pointer("/error/message"))
        .or_else(|| value.pointer("/error/description"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn rows(value: &Value) -> Vec<Value> {
    let data = unwrap_data(value);
    if let Some(items) = data.as_array() {
        return items.clone();
    }
    for pointer in [
        "/items",
        "/data",
        "/deployments",
        "/rows",
        "/data/items",
        "/data/data",
        "/data/deployments",
    ] {
        if let Some(items) = data.pointer(pointer).and_then(Value::as_array) {
            return items.clone();
        }
        if let Some(items) = value.pointer(pointer).and_then(Value::as_array) {
            return items.clone();
        }
    }
    Vec::new()
}

pub fn string_at_any(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| {
            v.as_str()
                .map(str::to_string)
                .or_else(|| v.as_i64().map(|n| n.to_string()))
                .or_else(|| v.as_u64().map(|n| n.to_string()))
        })
    })
}

pub fn status_string(value: &Value) -> Option<String> {
    value.get("status").and_then(|v| {
        v.as_str()
            .map(str::to_string)
            .or_else(|| v.as_i64().map(legacy_status_name))
            .or_else(|| {
                v.as_u64()
                    .and_then(|n| i64::try_from(n).ok())
                    .map(legacy_status_name)
            })
    })
}

fn legacy_status_name(status: i64) -> String {
    match status {
        0 => "pending",
        1 => "building",
        2 => "deploying",
        3 => "active",
        4 => "failed",
        5 => "stopped",
        _ => return format!("unknown_{status}"),
    }
    .to_string()
}

/// Best-effort check: does this raw stdout *look* like an error-shaped
/// envelope, even when full JSON parsing fails or yields an unknown shape?
///
/// Used by callers (e.g. `list_deployments::parse_list_deployments_cli_output`)
/// to disambiguate "transport-level failure" from "CLI reported an error
/// using a field shape we don't recognize yet". PR #149 / review #12.
///
/// Heuristic: presence of an `"error"` key or a `"status": "error"` token in
/// the raw stdout. Substring-only — does not require full JSON validity, so
/// it survives upstream envelopes that change field shape between versions.
pub fn looks_like_error_envelope(raw: &str) -> bool {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return false;
    }
    // Cheap textual fingerprints — avoids re-parsing JSON the caller has
    // already failed to parse. Either signal is sufficient.
    normalized.contains("\"status\":\"error\"")
        || normalized.contains("\"status\": \"error\"")
        || normalized.contains("\"error\":")
        || normalized.contains("\"error\" :")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_error_envelope_recognizes_status_error() {
        assert!(looks_like_error_envelope(
            r#"{"schema_version":"1","status":"error","error":{}}"#
        ));
        // With whitespace tolerance:
        assert!(looks_like_error_envelope(r#"{ "status": "error" }"#));
    }

    #[test]
    fn looks_like_error_envelope_recognizes_unknown_shape_with_error_key() {
        // Future CLI uses `error.slug` instead of `error.code`:
        assert!(looks_like_error_envelope(
            r#"{"error":{"slug":"resource.busy"}}"#
        ));
    }

    #[test]
    fn looks_like_error_envelope_rejects_success_envelopes() {
        assert!(!looks_like_error_envelope(
            r#"{"status":"ok","data":{"items":[]}}"#
        ));
        assert!(!looks_like_error_envelope(r#"{"items":[]}"#));
        assert!(!looks_like_error_envelope(""));
    }

    #[test]
    fn looks_like_error_envelope_rejects_word_in_unrelated_position() {
        // `error_summary` field that just *mentions* the word "error" must
        // not flip the heuristic — the substring needle requires the
        // `"error":` prefix shape.
        assert!(!looks_like_error_envelope(
            r#"{"data":{"error_summary":"none"}}"#
        ));
    }
}

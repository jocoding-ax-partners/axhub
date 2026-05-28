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

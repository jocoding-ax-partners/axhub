use serde_json::Value;

pub fn additional_context_for_payload(payload: &Value) -> Option<String> {
    let tool = payload.get("tool_name").and_then(Value::as_str)?;
    if !matches!(tool, "Edit" | "Write" | "MultiEdit" | "NotebookEdit") {
        return None;
    }
    let path = payload
        .pointer("/tool_input/file_path")
        .or_else(|| payload.pointer("/tool_input/path"))
        .or_else(|| payload.pointer("/tool_input/notebook_path"))
        .and_then(Value::as_str)?;
    if !is_source_write_path(path) || is_test_or_non_code_path(path) {
        return None;
    }
    Some(
        r#"<axhub-tdd-cycle>
[axhub hook | source-file write detected]
Observed: code change without a companion test in this turn.
Suggested: write a failing test first (RED), then minimal implementation (GREEN), then refactor.
Skip: AXHUB_DISABLE_HOOK=tdd-inject
</axhub-tdd-cycle>"#
            .to_string(),
    )
}

pub fn is_source_write_path(path: &str) -> bool {
    matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str()),
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

pub fn is_test_or_non_code_path(path: &str) -> bool {
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

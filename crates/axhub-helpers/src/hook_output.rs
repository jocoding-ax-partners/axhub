use serde_json::json;

pub fn session_start_context(text: &str) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": text,
        }
    })
    .to_string()
}

pub fn user_prompt_context(text: &str) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": text,
        }
    })
    .to_string()
}

/// Like [`user_prompt_context`], but also carries a top-level user-facing
/// `systemMessage` when `system_message` is `Some`. The hook layer is
/// systemMessage-injection only (spec 006: no interactive tools), so this is the
/// channel the once-per-project migration grace nudge rides on while the
/// agent-facing preflight stays in `additionalContext`. When `None`, the output
/// is byte-identical to [`user_prompt_context`].
pub fn user_prompt_context_with_system(text: &str, system_message: Option<&str>) -> String {
    let mut value = json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": text,
        }
    });
    if let Some(message) = system_message {
        value["systemMessage"] = json!(message);
    }
    value.to_string()
}

pub fn pre_tool_use_context(text: &str) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": text,
        }
    })
    .to_string()
}

pub fn post_tool_use_context(text: &str) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": text,
        }
    })
    .to_string()
}

pub fn pre_tool_use_ask(reason: &str) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}

pub fn pre_tool_use_deny(reason: &str) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}

pub fn pre_tool_use_allow() -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
        }
    })
    .to_string()
}

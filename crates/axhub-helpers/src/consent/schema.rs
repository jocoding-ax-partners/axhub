use std::fmt;

use super::jwt::ConsentBinding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSchemaError {
    MissingField(&'static str),
    MissingContext(&'static str),
    UnknownAction(String),
}

impl fmt::Display for BindingSchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "binding_schema:missing_field:{field}"),
            Self::MissingContext(key) => write!(f, "binding_schema:missing_context:{key}"),
            Self::UnknownAction(action) => write!(f, "binding_schema:unknown_action:{action}"),
        }
    }
}

impl std::error::Error for BindingSchemaError {}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

fn require_field(value: &str, field: &'static str) -> Result<(), BindingSchemaError> {
    if has_value(value) {
        Ok(())
    } else {
        Err(BindingSchemaError::MissingField(field))
    }
}

fn require_context(binding: &ConsentBinding, key: &'static str) -> Result<(), BindingSchemaError> {
    binding
        .context
        .get(key)
        .filter(|value| has_value(value))
        .map(|_| ())
        .ok_or(BindingSchemaError::MissingContext(key))
}

fn require_app(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_field(&binding.app_id, "app_id")
}

pub fn validate_binding_schema(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_field(&binding.tool_call_id, "tool_call_id")?;
    require_field(&binding.action, "action")?;

    match binding.action.as_str() {
        "deploy_create" => {
            require_app(binding)?;
            require_field(&binding.branch, "branch")?;
            require_field(&binding.commit_sha, "commit_sha")
        }
        "update_apply" | "deploy_logs_kill" => require_app(binding),
        "auth_login" => {
            require_field(&binding.profile, "profile")?;
            require_app(binding)?;
            require_field(&binding.branch, "branch")?;
            require_field(&binding.commit_sha, "commit_sha")
        }
        "env_set" | "env_delete" => {
            require_app(binding)?;
            require_context(binding, "key")
        }
        "apps_create" => require_context(binding, "source"),
        "apps_update" => {
            require_app(binding)?;
            require_context(binding, "slug")?;
            require_context(binding, "field")
        }
        "apps_delete" => {
            require_app(binding)?;
            require_context(binding, "slug")
        }
        "github_connect" => {
            require_app(binding)?;
            require_context(binding, "repo")?;
            require_context(binding, "branch")
        }
        "github_disconnect" => {
            require_app(binding)?;
            require_context(binding, "slug")
        }
        "deploy_cancel" => {
            require_app(binding)?;
            require_context(binding, "deployment_id")
        }
        "profile_add" => {
            require_context(binding, "profile")?;
            require_context(binding, "endpoint")
        }
        "profile_use" => require_context(binding, "profile"),
        "apis_call" => {
            require_context(binding, "endpoint_id")?;
            require_context(binding, "method")
        }
        other => Err(BindingSchemaError::UnknownAction(other.into())),
    }
}

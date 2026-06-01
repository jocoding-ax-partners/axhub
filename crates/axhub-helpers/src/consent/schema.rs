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
            // branch is NOT bound: the deploy targets a specific commit_sha
            // (the artifact), and `axhub deploy create` does not accept a
            // --branch flag, so the command can't echo it for verification.
            // branch stays informational (resolve/preview) only.
            require_app(binding)?;
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

#[cfg(test)]
mod tests {
    use super::super::jwt::ConsentBinding;
    use super::{validate_binding_schema, BindingSchemaError};
    use std::collections::HashMap;

    fn deploy_binding(branch: &str, commit_sha: &str) -> ConsentBinding {
        ConsentBinding {
            tool_call_id: "sess:tc".into(),
            action: "deploy_create".into(),
            app_id: "paydrop".into(),
            profile: "prod".into(),
            branch: branch.into(),
            commit_sha: commit_sha.into(),
            context: HashMap::new(),
            synthesized_by_helper: false,
        }
    }

    #[test]
    fn deploy_create_does_not_require_branch() {
        // Option 2: deploy_create binds on commit_sha (the artifact), not the
        // branch label — `axhub deploy create` has no --branch flag to echo, so
        // a binding with branch omitted ("") must validate.
        assert!(validate_binding_schema(&deploy_binding("", "abc1234")).is_ok());
    }

    #[test]
    fn deploy_create_still_requires_commit_sha() {
        assert_eq!(
            validate_binding_schema(&deploy_binding("main", "")),
            Err(BindingSchemaError::MissingField("commit_sha"))
        );
    }

    #[test]
    fn deploy_create_with_branch_remains_valid() {
        // Backward-compat: a binding still carrying branch validates fine.
        assert!(validate_binding_schema(&deploy_binding("main", "abc1234")).is_ok());
    }

    #[test]
    fn auth_login_still_requires_branch() {
        // Regression guard: Option 2 dropped the branch requirement only for
        // deploy_create. auth_login must still reject an empty branch.
        let binding = ConsentBinding {
            tool_call_id: "sess:tc".into(),
            action: "auth_login".into(),
            app_id: "_".into(),
            profile: "default".into(),
            branch: String::new(),
            commit_sha: "_".into(),
            context: HashMap::new(),
            synthesized_by_helper: false,
        };
        assert_eq!(
            validate_binding_schema(&binding),
            Err(BindingSchemaError::MissingField("branch"))
        );
    }

    #[test]
    fn consent_binding_deserializes_without_branch() {
        // serde(default) lets the consent-mint payload omit branch entirely.
        let json = r#"{"tool_call_id":"s:t","action":"deploy_create","app_id":"paydrop","profile":"prod","commit_sha":"abc1234","context":{}}"#;
        let binding: ConsentBinding =
            serde_json::from_str(json).expect("deserialize without branch");
        assert_eq!(binding.branch, "");
        assert!(validate_binding_schema(&binding).is_ok());
    }
}

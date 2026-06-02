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

fn require_concrete_context(
    binding: &ConsentBinding,
    key: &'static str,
) -> Result<(), BindingSchemaError> {
    binding
        .context
        .get(key)
        .filter(|value| has_value(value) && value.as_str() != "<active>")
        .map(|_| ())
        .ok_or(BindingSchemaError::MissingContext(key))
}

fn context_list(binding: &ConsentBinding, key: &str) -> Vec<String> {
    binding
        .context
        .get(key)
        .map(|fields| {
            fields
                .split(',')
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn require_app(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_field(&binding.app_id, "app_id")
}

fn require_tenant(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_context(binding, "tenant")
}

fn require_any_context(
    binding: &ConsentBinding,
    keys: &'static [&'static str],
    fallback: &'static str,
) -> Result<(), BindingSchemaError> {
    if keys.iter().any(|key| {
        binding
            .context
            .get(*key)
            .filter(|value| has_value(value))
            .is_some()
    }) {
        Ok(())
    } else {
        Err(BindingSchemaError::MissingContext(fallback))
    }
}

fn require_config_identity(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_context(binding, "config_digest")
}

fn require_credentials_identity(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_context(binding, "credentials_digest")
}

fn require_items_identity(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_context(binding, "items_digest")
}

fn require_data_payload_identity(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_context(binding, "source")?;
    match binding.context.get("source").map(String::as_str) {
        Some("body") => require_context(binding, "body_digest"),
        Some("body_file") => require_context(binding, "body_digest"),
        Some("batch") => require_context(binding, "batch_digest"),
        _ => require_any_context(
            binding,
            &["body_digest", "batch_digest"],
            "payload_identity",
        ),
    }
}

fn require_connector_update_context(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_context(binding, "connector_id")?;
    require_tenant(binding)?;
    require_context(binding, "fields")?;

    let fields = context_list(binding, "fields");
    if fields.is_empty() {
        return Err(BindingSchemaError::MissingContext("fields"));
    }

    for field in fields {
        match field.as_str() {
            "config" => {
                require_context(binding, "source")?;
                require_config_identity(binding)?;
            }
            "description" => require_context(binding, "description_digest")?,
            "enabled" => require_context(binding, "enabled")?,
            "disabled" => require_context(binding, "disabled")?,
            _ => return Err(BindingSchemaError::MissingContext("known_field")),
        }
    }
    Ok(())
}

fn require_apps_update_context(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_app(binding)?;
    require_context(binding, "slug")?;
    require_context(binding, "fields")?;
    let fields = context_list(binding, "fields");
    if fields.is_empty() {
        return Err(BindingSchemaError::MissingContext("fields"));
    }
    for field in fields {
        let context_key: &'static str = match field.as_str() {
            "name" => "name",
            "description" => "description",
            "visibility" => "visibility",
            "resource_tier" => "resource_tier",
            "subdomain" => "subdomain",
            "clear_subdomain" => "clear_subdomain",
            "auth_mode" => "auth_mode",
            "category_id" => "category_id",
            "clear_category" => "clear_category",
            "data_scopes" => "data_scopes",
            "icon_dark_url" => "icon_dark_url",
            "icon_url" => "icon_url",
            _ => return Err(BindingSchemaError::MissingContext("known_field")),
        };
        require_context(binding, context_key)?;
    }
    Ok(())
}

pub fn validate_binding_schema(binding: &ConsentBinding) -> Result<(), BindingSchemaError> {
    require_field(&binding.tool_call_id, "tool_call_id")?;
    require_field(&binding.action, "action")?;

    match binding.action.as_str() {
        "deploy_create" => {
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
        "apps_update" => require_apps_update_context(binding),
        "apps_delete" => {
            require_app(binding)?;
            require_context(binding, "slug")
        }
        "apps_fork" => {
            require_app(binding)?;
            require_context(binding, "source")?;
            require_context(binding, "slug")?;
            require_context(binding, "subdomain")?;
            require_context(binding, "tenant")?;
            require_context(binding, "name")?;
            require_context(binding, "template")?;
            require_context(binding, "repo_public")
        }
        "apps_suspend" | "apps_resume" => require_app(binding),
        "publish_submit" => {
            require_app(binding)?;
            require_context(binding, "note_length")?;
            require_context(binding, "note_digest")
        }
        "auth_oauth_client_create" => {
            require_app(binding)?;
            require_context(binding, "name")?;
            require_context(binding, "type")?;
            require_context(binding, "redirect_uris")?;
            require_context(binding, "scopes")?;
            require_context(binding, "grant_types")
        }
        "auth_oauth_revoke" => {
            require_context(binding, "target")?;
            require_context(binding, "client_id")
        }
        "auth_oauth_consent_revoke" => require_context(binding, "client_id"),
        "auth_logout" => require_concrete_context(binding, "profile"),
        "auth_pat_issue" => {
            require_context(binding, "name")?;
            require_context(binding, "expires_in_days")?;
            require_context(binding, "use")?;
            require_context(binding, "no_save")?;
            require_context(binding, "show_token")
        }
        "auth_pat_revoke" => require_context(binding, "pat_id"),
        "auth_pat_use" => {
            require_context(binding, "pat_id")?;
            require_concrete_context(binding, "profile")
        }
        "auth_pat_unset" => {
            require_context(binding, "target")?;
            require_concrete_context(binding, "profile")
        }
        "auth_pat_rotate" => require_context(binding, "name"),
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
        "deploy_rollback" => {
            require_app(binding)?;
            require_context(binding, "from_deployment")
        }
        "invitation_send" => {
            require_context(binding, "email")?;
            require_tenant(binding)
        }
        "invitation_bulk" => {
            require_context(binding, "source")?;
            require_tenant(binding)
        }
        "invitation_cancel" | "invitation_resend" => {
            require_context(binding, "invitation_id")?;
            require_tenant(binding)
        }
        "access_grant" | "access_revoke" => require_app(binding),
        "access_invite" | "access_uninvite" => {
            require_app(binding)?;
            require_context(binding, "user")
        }
        "tables_create" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "column")
        }
        "tables_drop" => {
            require_app(binding)?;
            require_context(binding, "table")
        }
        "tables_columns_add" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "name")?;
            require_context(binding, "type")
        }
        "tables_columns_remove" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "name")
        }
        "tables_grants_issue" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "principal_id")?;
            require_context(binding, "actions")
        }
        "tables_grants_revoke" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "grant_id")
        }
        "data_insert" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_data_payload_identity(binding)
        }
        "data_update" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "row_id")?;
            require_data_payload_identity(binding)
        }
        "data_delete" => {
            require_app(binding)?;
            require_context(binding, "table")?;
            require_context(binding, "row_id")
        }
        "connector_create" => {
            require_context(binding, "name")?;
            require_context(binding, "engine")?;
            require_context(binding, "source")?;
            require_config_identity(binding)?;
            require_credentials_identity(binding)?;
            require_tenant(binding)
        }
        "connector_update" => require_connector_update_context(binding),
        "connector_credentials_set" => {
            require_context(binding, "connector_id")?;
            require_context(binding, "source")?;
            require_credentials_identity(binding)?;
            require_tenant(binding)
        }
        "connector_delete" => {
            require_context(binding, "connector_id")?;
            require_tenant(binding)
        }
        "resource_namespace_create" => {
            require_context(binding, "name")?;
            require_tenant(binding)
        }
        "resource_rename" => {
            require_context(binding, "resource_id")?;
            require_context(binding, "name")?;
            require_tenant(binding)
        }
        "resource_move" => {
            require_context(binding, "resource_id")?;
            require_context(binding, "parent_id")?;
            require_tenant(binding)
        }
        "resource_bulk_register" => {
            require_context(binding, "connector_id")?;
            require_context(binding, "source")?;
            require_items_identity(binding)?;
            require_tenant(binding)
        }
        "resource_delete" => {
            require_context(binding, "resource_id")?;
            require_tenant(binding)
        }
        "resource_tag_attach" | "resource_tag_detach" => {
            require_context(binding, "resource_id")?;
            require_context(binding, "tag_id")?;
            require_tenant(binding)
        }
        other => Err(BindingSchemaError::UnknownAction(other.into())),
    }
}

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::consent::{validate_binding_schema, ConsentBinding};
use crate::telemetry::emit_meta_envelope;

pub const BOOTSTRAP_STATE_VERSION: &str = "bootstrap-state/v1";
pub const BOOTSTRAP_RECORD_SCHEMA_VERSION: &str = "bootstrap-record/v1";
pub const BOOTSTRAP_STATE_RELATIVE_PATH: &str = ".axhub/bootstrap.state.json";
pub const BOOTSTRAP_TELEMETRY_SCHEMA_VERSION: &str = "bootstrap-telemetry/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapState {
    TemplateRequired,
    ConflictExistingFiles,
    GitInitRequired,
    FirstCommitRequired,
    SubdomainCollision,
    AlreadyDeployed,
    ConsentRequiredAppsCreate,
    ConsentRequiredDeployCreate,
    BackendContractMissingDefaults,
    IdempotencyUnavailable,
    AppRegistered,
    Deploying,
    Deployed,
}

impl BootstrapState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TemplateRequired => "template_required",
            Self::ConflictExistingFiles => "conflict_existing_files",
            Self::GitInitRequired => "git_init_required",
            Self::FirstCommitRequired => "first_commit_required",
            Self::SubdomainCollision => "subdomain_collision",
            Self::AlreadyDeployed => "already_deployed",
            Self::ConsentRequiredAppsCreate => "consent_required_apps_create",
            Self::ConsentRequiredDeployCreate => "consent_required_deploy_create",
            Self::BackendContractMissingDefaults => "backend_contract_missing_defaults",
            Self::IdempotencyUnavailable => "idempotency_unavailable",
            Self::AppRegistered => "app_registered",
            Self::Deploying => "deploying",
            Self::Deployed => "deployed",
        }
    }

    fn is_user_decision(self) -> bool {
        matches!(
            self,
            Self::TemplateRequired
                | Self::ConflictExistingFiles
                | Self::GitInitRequired
                | Self::FirstCommitRequired
                | Self::SubdomainCollision
                | Self::AlreadyDeployed
                | Self::ConsentRequiredAppsCreate
                | Self::ConsentRequiredDeployCreate
                | Self::BackendContractMissingDefaults
                | Self::IdempotencyUnavailable
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppsCreateSuccess {
    pub app_id: String,
    pub app_slug: String,
    pub subdomain: String,
    pub domain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppsCreateDecision {
    Registered(AppsCreateSuccess),
    Stop {
        state: BootstrapState,
        reason: String,
        suggested_subdomain: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub hash: String,
    pub event: String,
    pub command_argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent_binding: Option<ConsentBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedAction {
    pub id: String,
    pub hash: String,
    pub event: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapStateFile {
    pub version: String,
    pub state: BootstrapState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdomain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_path: Option<String>,
    pub git_initialized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_deploy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_create_attempted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_action: Option<PendingAction>,
    pub completed_actions: Vec<CompletedAction>,
    pub updated_at: String,
}

impl BootstrapStateFile {
    fn new(state: BootstrapState, manifest_path: Option<String>) -> Self {
        Self {
            version: BOOTSTRAP_STATE_VERSION.into(),
            state,
            app_id: None,
            app_slug: None,
            subdomain: None,
            domain_id: None,
            manifest_path,
            git_initialized: false,
            last_deploy_id: None,
            deploy_create_attempted_at: None,
            pending_action: None,
            completed_actions: vec![],
            updated_at: now_ts(),
        }
    }

    fn touch(&mut self) {
        self.updated_at = now_ts();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapOutput {
    pub state: BootstrapState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent_binding: Option<ConsentBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_action_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl BootstrapOutput {
    fn state(state: BootstrapState) -> Self {
        let user_decision = state.is_user_decision().then(|| state.as_str().to_string());
        Self {
            state,
            next_action: None,
            user_decision,
            command: None,
            consent_binding: None,
            binding_hash: None,
            pending_action_id: None,
            pending_action_hash: None,
            idempotency_key: None,
            retry_policy: None,
            reason: None,
        }
    }

    fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    fn with_action(mut self, action: impl Into<String>, command: Vec<String>) -> Self {
        self.next_action = Some(action.into());
        self.command = Some(command);
        self
    }

    fn with_pending(mut self, pending: &PendingAction) -> Self {
        self.pending_action_id = Some(pending.id.clone());
        self.pending_action_hash = Some(pending.hash.clone());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapRun {
    pub output: BootstrapOutput,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestInfo {
    path: String,
    slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BootstrapRecordEnvelope {
    schema_version: String,
    pending_action_id: String,
    pending_action_hash: String,
    command_argv: Vec<String>,
    exit_code: i32,
    stdout_json: Value,
    #[allow(dead_code)]
    stderr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootstrapArgs {
    json: bool,
    dry_run: bool,
    auto_chain: bool,
    record_event: Option<String>,
}

pub fn run_bootstrap(args: &[String], stdin: Option<&str>) -> BootstrapRun {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(reason) => return fail_64(reason),
    };

    if args.record_event.is_some() && (args.dry_run || args.auto_chain) {
        return fail_64("record_mode_cannot_combine_with_plan_flags");
    }

    if let Some(event) = args.record_event.as_deref() {
        return with_bootstrap_phase("record_event", || {
            record_event(event, stdin.unwrap_or_default())
        });
    }

    with_bootstrap_phase("plan_next", || {
        plan_next(args.dry_run || !args.auto_chain, args.auto_chain)
    })
}

fn with_bootstrap_phase<F>(phase: &'static str, run: F) -> BootstrapRun
where
    F: FnOnce() -> BootstrapRun,
{
    let started = Instant::now();
    emit_bootstrap_marker(
        "bootstrap_phase_start",
        [
            ("phase", Value::String(phase.into())),
            ("state", Value::String("unknown".into())),
            ("outcome", Value::String("started".into())),
        ],
    );
    let result = run();
    emit_bootstrap_marker(
        "bootstrap_phase_end",
        [
            ("phase", Value::String(phase.into())),
            ("state", Value::String(result.output.state.as_str().into())),
            (
                "outcome",
                Value::String(if result.exit_code == 0 { "ok" } else { "stop" }.into()),
            ),
            ("elapsed_ms", json!(started.elapsed().as_millis() as u64)),
        ],
    );
    result
}

fn emit_bootstrap_re_entry(state: BootstrapState) {
    emit_bootstrap_marker(
        "bootstrap_re_entry_at_state",
        [
            ("phase", Value::String("plan_next".into())),
            ("state", Value::String(state.as_str().into())),
            ("outcome", Value::String("re_entry".into())),
        ],
    );
}

fn emit_consent_synthesized_by_helper(event: &str, state: BootstrapState, pending: &PendingAction) {
    emit_bootstrap_marker(
        "consent_synthesized_by_helper",
        [
            ("phase", Value::String("plan_remote_action".into())),
            ("state", Value::String(state.as_str().into())),
            ("outcome", Value::String("planned".into())),
            (
                "decision_class",
                Value::String("remote_destructive_plan".into()),
            ),
            ("record_event", Value::String(event.into())),
            (
                "retry_policy",
                Value::String(if pending.event == "apps_create" {
                    "no_retry_without_confirmed_idempotency".into()
                } else {
                    "none".into()
                }),
            ),
        ],
    );
}

fn emit_bootstrap_marker<const N: usize>(event: &'static str, fields: [(&'static str, Value); N]) {
    let mut envelope = Map::new();
    envelope.insert("event".into(), Value::String(event.into()));
    envelope.insert(
        "schema_version".into(),
        Value::String(BOOTSTRAP_TELEMETRY_SCHEMA_VERSION.into()),
    );
    for (key, value) in fields {
        envelope.insert(key.into(), value);
    }
    let _ = emit_meta_envelope(envelope);
}

fn parse_args(args: &[String]) -> Result<BootstrapArgs, String> {
    let mut parsed = BootstrapArgs {
        json: false,
        dry_run: false,
        auto_chain: false,
        record_event: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => parsed.json = true,
            "--dry-run" | "--plan-only" => parsed.dry_run = true,
            "--auto-chain" => parsed.auto_chain = true,
            "--record" => {
                i += 1;
                let Some(event) = args.get(i) else {
                    return Err("record_event_missing".into());
                };
                if event.starts_with("--") {
                    return Err("record_event_missing".into());
                }
                parsed.record_event = Some(event.clone());
            }
            other => return Err(format!("unknown_option:{other}")),
        }
        i += 1;
    }
    Ok(parsed)
}

fn plan_next(plan_only: bool, persist_plan: bool) -> BootstrapRun {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            return stop(
                BootstrapState::BackendContractMissingDefaults,
                err.to_string(),
            )
        }
    };
    let manifest = read_manifest(&cwd);
    let Some(manifest) = manifest else {
        return BootstrapRun {
            output: BootstrapOutput::state(BootstrapState::TemplateRequired)
                .with_reason("manifest_missing"),
            exit_code: 65,
        };
    };

    let state = match load_state(&cwd) {
        Ok(state) => state,
        Err(reason) => {
            return BootstrapRun {
                output: BootstrapOutput::state(BootstrapState::BackendContractMissingDefaults)
                    .with_reason(reason),
                exit_code: 65,
            }
        }
    };

    if let Some(state) = state {
        emit_bootstrap_re_entry(state.state);
        if state.last_deploy_id.is_some() && matches!(state.state, BootstrapState::Deployed) {
            return BootstrapRun {
                output: BootstrapOutput::state(BootstrapState::AlreadyDeployed)
                    .with_reason("last_deploy_present"),
                exit_code: 65,
            };
        }
        if let Some(pending) = state.pending_action.as_ref() {
            return output_pending(&state, pending, 0);
        }
        if state_is_terminal_stop(state.state) {
            return BootstrapRun {
                output: BootstrapOutput::state(state.state)
                    .with_reason("state_requires_user_decision"),
                exit_code: 65,
            };
        }
        if state.app_id.is_some() && state.subdomain.is_some() && state.domain_id.is_some() {
            return plan_after_app_registered(&cwd, state, plan_only, persist_plan);
        }
    }

    plan_apps_create(&manifest, plan_only, persist_plan)
}

fn state_is_terminal_stop(state: BootstrapState) -> bool {
    matches!(
        state,
        BootstrapState::SubdomainCollision
            | BootstrapState::AlreadyDeployed
            | BootstrapState::BackendContractMissingDefaults
            | BootstrapState::IdempotencyUnavailable
    )
}

fn plan_after_app_registered(
    cwd: &Path,
    mut state: BootstrapStateFile,
    _plan_only: bool,
    persist_plan: bool,
) -> BootstrapRun {
    if !cwd.join(".git").exists() {
        return BootstrapRun {
            output: BootstrapOutput::state(BootstrapState::GitInitRequired)
                .with_action("git_init", vec!["git".into(), "init".into()])
                .with_reason("git_repository_missing"),
            exit_code: 65,
        };
    }
    state.git_initialized = true;
    if git_output(cwd, &["rev-parse", "--verify", "HEAD"]).is_none() {
        return BootstrapRun {
            output: BootstrapOutput::state(BootstrapState::FirstCommitRequired)
                .with_action(
                    "first_commit",
                    vec![
                        "git".into(),
                        "commit".into(),
                        "-m".into(),
                        "Initial axhub bootstrap".into(),
                    ],
                )
                .with_reason("git_head_missing"),
            exit_code: 65,
        };
    }

    let branch = git_output(cwd, &["branch", "--show-current"]).unwrap_or_else(|| "main".into());
    let commit = git_output(cwd, &["rev-parse", "HEAD"]).unwrap_or_else(|| "HEAD".into());
    let app = state
        .app_slug
        .clone()
        .or_else(|| state.app_id.clone())
        .unwrap_or_else(|| "".into());
    let command = vec![
        "axhub".into(),
        "deploy".into(),
        "create".into(),
        "--app".into(),
        app.clone(),
        "--branch".into(),
        branch.clone(),
        "--commit".into(),
        commit.clone(),
        "--json".into(),
    ];
    let binding = ConsentBinding {
        tool_call_id: "pending".into(),
        action: "deploy_create".into(),
        app_id: app,
        profile: std::env::var("AXHUB_PROFILE").unwrap_or_default(),
        branch,
        commit_sha: commit,
        context: HashMap::new(),
        synthesized_by_helper: true,
    };
    plan_remote_action(
        cwd,
        &mut state,
        BootstrapState::ConsentRequiredDeployCreate,
        "deploy_create",
        command,
        binding,
        persist_plan,
    )
}

fn plan_apps_create(manifest: &ManifestInfo, plan_only: bool, persist_plan: bool) -> BootstrapRun {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            return stop(
                BootstrapState::BackendContractMissingDefaults,
                err.to_string(),
            )
        }
    };
    let mut state = BootstrapStateFile::new(
        BootstrapState::ConsentRequiredAppsCreate,
        Some(manifest.path.clone()),
    );
    let command = vec![
        "axhub".into(),
        "apps".into(),
        "create".into(),
        "--from-file".into(),
        manifest.path.clone(),
        "--json".into(),
    ];
    let mut context = HashMap::new();
    context.insert("source".into(), manifest.path.clone());
    let binding = ConsentBinding {
        tool_call_id: "pending".into(),
        action: "apps_create".into(),
        app_id: String::new(),
        profile: std::env::var("AXHUB_PROFILE").unwrap_or_default(),
        branch: String::new(),
        commit_sha: String::new(),
        context,
        synthesized_by_helper: true,
    };
    plan_remote_action(
        &cwd,
        &mut state,
        BootstrapState::ConsentRequiredAppsCreate,
        "apps_create",
        command,
        binding,
        persist_plan && !plan_only,
    )
}

fn plan_remote_action(
    cwd: &Path,
    state: &mut BootstrapStateFile,
    bootstrap_state: BootstrapState,
    event: &str,
    command: Vec<String>,
    binding: ConsentBinding,
    persist_plan: bool,
) -> BootstrapRun {
    if let Err(err) = validate_binding_schema(&binding) {
        return stop(
            BootstrapState::BackendContractMissingDefaults,
            err.to_string(),
        );
    }
    let binding_hash = stable_hash(&json!({"binding": binding, "command": command}));
    let pending_hash = stable_hash(&json!({
        "event": event,
        "binding_hash": binding_hash,
        "command_argv": command,
    }));
    let pending = PendingAction {
        id: format!("{event}:{}", &binding_hash[..16]),
        hash: pending_hash,
        event: event.into(),
        command_argv: command.clone(),
        consent_binding: Some(binding.clone()),
        binding_hash: Some(binding_hash.clone()),
        created_at: now_ts(),
    };
    state.state = bootstrap_state;
    state.pending_action = Some(pending.clone());
    state.touch();
    if persist_plan {
        if let Err(err) = save_state(cwd, state) {
            return stop(BootstrapState::BackendContractMissingDefaults, err);
        }
    }
    emit_consent_synthesized_by_helper(event, bootstrap_state, &pending);
    BootstrapRun {
        output: BootstrapOutput::state(bootstrap_state)
            .with_action(event, command)
            .with_pending(&pending)
            .with_retry_policy_for(event)
            .with_binding(binding, binding_hash),
        exit_code: 0,
    }
}

trait BootstrapOutputExt {
    fn with_binding(self, binding: ConsentBinding, hash: String) -> Self;
    fn with_retry_policy_for(self, event: &str) -> Self;
}

impl BootstrapOutputExt for BootstrapOutput {
    fn with_binding(mut self, binding: ConsentBinding, hash: String) -> Self {
        self.consent_binding = Some(binding);
        self.binding_hash = Some(hash);
        self
    }

    fn with_retry_policy_for(mut self, event: &str) -> Self {
        if event == "apps_create" {
            self.retry_policy = Some("no_retry_without_confirmed_idempotency".into());
        }
        self
    }
}

fn output_pending(
    state: &BootstrapStateFile,
    pending: &PendingAction,
    exit_code: i32,
) -> BootstrapRun {
    let mut output = BootstrapOutput::state(state.state)
        .with_action(pending.event.clone(), pending.command_argv.clone())
        .with_pending(pending);
    output.consent_binding = pending.consent_binding.clone();
    output.binding_hash = pending.binding_hash.clone();
    if pending.event == "apps_create" {
        output.retry_policy = Some("no_retry_without_confirmed_idempotency".into());
    }
    if pending.consent_binding.is_some() {
        emit_consent_synthesized_by_helper(&pending.event, state.state, pending);
    }
    BootstrapRun { output, exit_code }
}

fn record_event(event: &str, stdin: &str) -> BootstrapRun {
    if !matches!(event, "apps_create" | "deploy_create") {
        return fail_64("record_event_unknown");
    }
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            return stop(
                BootstrapState::BackendContractMissingDefaults,
                err.to_string(),
            )
        }
    };
    let envelope = match parse_record_envelope(stdin) {
        Ok(envelope) => envelope,
        Err(reason) => return fail_64(reason),
    };
    let mut state = match load_state(&cwd) {
        Ok(Some(state)) => state,
        Ok(None) => return fail_64("record_duplicate_or_no_pending_action"),
        Err(reason) => {
            return BootstrapRun {
                output: BootstrapOutput::state(BootstrapState::BackendContractMissingDefaults)
                    .with_reason(reason),
                exit_code: 65,
            }
        }
    };
    let Some(pending) = state.pending_action.clone() else {
        return fail_64("record_duplicate_or_no_pending_action");
    };
    if pending.event != event {
        return fail_64("record_out_of_order");
    }
    if pending.id != envelope.pending_action_id || pending.hash != envelope.pending_action_hash {
        return fail_64("record_pending_action_mismatch");
    }
    if pending.command_argv != envelope.command_argv {
        return fail_64("record_command_argv_mismatch");
    }

    match event {
        "apps_create" => record_apps_create(&cwd, &mut state, &pending, envelope),
        "deploy_create" => record_deploy_create(&cwd, &mut state, &pending, envelope),
        _ => unreachable!(),
    }
}

fn parse_record_envelope(stdin: &str) -> Result<BootstrapRecordEnvelope, String> {
    let value: Value =
        serde_json::from_str(stdin).map_err(|_| "record_envelope_invalid_json".to_string())?;
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        return Err("record_schema_version_missing".into());
    }
    let envelope: BootstrapRecordEnvelope =
        serde_json::from_value(value).map_err(|_| "record_envelope_invalid_shape".to_string())?;
    if envelope.schema_version != BOOTSTRAP_RECORD_SCHEMA_VERSION {
        return Err("record_schema_version_mismatch".into());
    }
    Ok(envelope)
}

fn record_apps_create(
    cwd: &Path,
    state: &mut BootstrapStateFile,
    pending: &PendingAction,
    envelope: BootstrapRecordEnvelope,
) -> BootstrapRun {
    match interpret_apps_create_result(envelope.exit_code, &envelope.stdout_json) {
        AppsCreateDecision::Registered(app) => {
            state.state = BootstrapState::AppRegistered;
            state.app_id = Some(app.app_id);
            state.app_slug = Some(app.app_slug);
            state.subdomain = Some(app.subdomain);
            state.domain_id = Some(app.domain_id);
            complete_pending(state, pending);
            if let Err(err) = save_state(cwd, state) {
                return stop(BootstrapState::BackendContractMissingDefaults, err);
            }
            BootstrapRun {
                output: BootstrapOutput::state(BootstrapState::AppRegistered),
                exit_code: 0,
            }
        }
        AppsCreateDecision::Stop {
            state: stop_state,
            reason,
            suggested_subdomain,
        } => {
            state.state = stop_state;
            complete_pending(state, pending);
            if let Err(err) = save_state(cwd, state) {
                return stop(BootstrapState::BackendContractMissingDefaults, err);
            }
            let mut output = BootstrapOutput::state(stop_state).with_reason(reason);
            if let Some(suggested) = suggested_subdomain {
                output.next_action = Some("choose_subdomain".into());
                output.command = Some(vec![
                    "axhub".into(),
                    "apps".into(),
                    "create".into(),
                    "--subdomain".into(),
                    suggested,
                ]);
            }
            BootstrapRun {
                output,
                exit_code: 65,
            }
        }
    }
}

fn record_deploy_create(
    cwd: &Path,
    state: &mut BootstrapStateFile,
    pending: &PendingAction,
    envelope: BootstrapRecordEnvelope,
) -> BootstrapRun {
    if envelope.exit_code != 0 {
        state.state = BootstrapState::IdempotencyUnavailable;
        complete_pending(state, pending);
        if let Err(err) = save_state(cwd, state) {
            return stop(BootstrapState::BackendContractMissingDefaults, err);
        }
        return BootstrapRun {
            output: BootstrapOutput::state(BootstrapState::IdempotencyUnavailable)
                .with_reason("deploy_create_failed_without_retry_contract"),
            exit_code: 65,
        };
    }
    let deployment_id = string_at(&envelope.stdout_json, &["/deployment_id", "/id"]);
    let Some(deployment_id) = deployment_id else {
        state.state = BootstrapState::BackendContractMissingDefaults;
        complete_pending(state, pending);
        if let Err(err) = save_state(cwd, state) {
            return stop(BootstrapState::BackendContractMissingDefaults, err);
        }
        return BootstrapRun {
            output: BootstrapOutput::state(BootstrapState::BackendContractMissingDefaults)
                .with_reason("deploy_create_missing_deployment_id"),
            exit_code: 65,
        };
    };
    state.state = BootstrapState::Deploying;
    state.last_deploy_id = Some(deployment_id);
    state.deploy_create_attempted_at = Some(now_ts());
    complete_pending(state, pending);
    if let Err(err) = save_state(cwd, state) {
        return stop(BootstrapState::BackendContractMissingDefaults, err);
    }
    BootstrapRun {
        output: BootstrapOutput::state(BootstrapState::Deploying),
        exit_code: 0,
    }
}

fn complete_pending(state: &mut BootstrapStateFile, pending: &PendingAction) {
    state.completed_actions.push(CompletedAction {
        id: pending.id.clone(),
        hash: pending.hash.clone(),
        event: pending.event.clone(),
        recorded_at: now_ts(),
    });
    state.pending_action = None;
    state.touch();
}

pub fn interpret_apps_create_result(exit_code: i32, stdout_json: &Value) -> AppsCreateDecision {
    if exit_code == 0 {
        let app_id = string_at(stdout_json, &["/app_id", "/id"]);
        let app_slug = string_at(stdout_json, &["/app_slug", "/slug"]);
        let subdomain = string_at(stdout_json, &["/subdomain"]);
        let domain_id = string_at(stdout_json, &["/domain_id"]);
        if let (Some(app_id), Some(app_slug), Some(subdomain), Some(domain_id)) =
            (app_id, app_slug, subdomain, domain_id)
        {
            return AppsCreateDecision::Registered(AppsCreateSuccess {
                app_id,
                app_slug,
                subdomain,
                domain_id,
            });
        }
        return AppsCreateDecision::Stop {
            state: BootstrapState::BackendContractMissingDefaults,
            reason: "apps_create_missing_server_defaults".into(),
            suggested_subdomain: None,
        };
    }

    let code = string_at(stdout_json, &["/error/code", "/code"]);
    if code.as_deref() == Some("subdomain_collision") || exit_code == 422 {
        return AppsCreateDecision::Stop {
            state: BootstrapState::SubdomainCollision,
            reason: "subdomain_collision".into(),
            suggested_subdomain: string_at(
                stdout_json,
                &["/suggestion/subdomain", "/suggested_subdomain"],
            ),
        };
    }

    AppsCreateDecision::Stop {
        state: BootstrapState::IdempotencyUnavailable,
        reason: "apps_create_retry_blocked_without_idempotency".into(),
        suggested_subdomain: None,
    }
}

fn read_manifest(cwd: &Path) -> Option<ManifestInfo> {
    ["apphub.yaml", "axhub.yaml"].into_iter().find_map(|path| {
        let raw = fs::read_to_string(cwd.join(path)).ok()?;
        Some(ManifestInfo {
            path: path.into(),
            slug: parse_manifest_slug(&raw),
        })
    })
}

fn parse_manifest_slug(raw: &str) -> Option<String> {
    for key in ["app_slug", "slug", "name"] {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((candidate_key, value)) = trimmed.split_once(':') else {
                continue;
            };
            if candidate_key.trim() != key {
                continue;
            }
            let value = value
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn state_path(cwd: &Path) -> PathBuf {
    cwd.join(BOOTSTRAP_STATE_RELATIVE_PATH)
}

fn load_state(cwd: &Path) -> Result<Option<BootstrapStateFile>, String> {
    let path = state_path(cwd);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|err| format!("state_read_failed:{err}"))?;
    let state: BootstrapStateFile =
        serde_json::from_str(&raw).map_err(|err| format!("state_corrupt:{err}"))?;
    if state.version != BOOTSTRAP_STATE_VERSION {
        return Err("state_version_mismatch".into());
    }
    Ok(Some(state))
}

fn save_state(cwd: &Path, state: &BootstrapStateFile) -> Result<(), String> {
    let path = state_path(cwd);
    let parent = path
        .parent()
        .ok_or_else(|| "state_parent_missing".to_string())?;
    let first_write = !path.exists();
    fs::create_dir_all(parent).map_err(|err| format!("state_dir_create_failed:{err}"))?;
    let tmp = path.with_extension("json.tmp");
    let raw =
        serde_json::to_vec_pretty(state).map_err(|err| format!("state_encode_failed:{err}"))?;
    fs::write(&tmp, raw).map_err(|err| format!("state_write_failed:{err}"))?;
    fs::rename(&tmp, &path).map_err(|err| format!("state_rename_failed:{err}"))?;
    if first_write {
        append_gitignore(cwd)?;
    }
    Ok(())
}

fn append_gitignore(cwd: &Path) -> Result<(), String> {
    let path = cwd.join(".gitignore");
    let mut raw = fs::read_to_string(&path).unwrap_or_default();
    if raw
        .lines()
        .any(|line| line.trim() == BOOTSTRAP_STATE_RELATIVE_PATH)
    {
        return Ok(());
    }
    if !raw.is_empty() && !raw.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str(BOOTSTRAP_STATE_RELATIVE_PATH);
    raw.push('\n');
    fs::write(&path, raw).map_err(|err| format!("gitignore_write_failed:{err}"))
}

fn stable_hash(value: &Value) -> String {
    let raw = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(raw);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").ok();
    }
    out
}

fn string_at(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers.iter().find_map(|pointer| {
        let found = value.pointer(pointer)?;
        match found {
            Value::String(s) => {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    })
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn fail_64(reason: impl Into<String>) -> BootstrapRun {
    BootstrapRun {
        output: BootstrapOutput::state(BootstrapState::BackendContractMissingDefaults)
            .with_reason(reason),
        exit_code: 64,
    }
}

fn stop(state: BootstrapState, reason: impl Into<String>) -> BootstrapRun {
    BootstrapRun {
        output: BootstrapOutput::state(state).with_reason(reason),
        exit_code: 65,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_state_strings_cover_terminal_and_progress_states() {
        let cases = [
            (
                BootstrapState::ConflictExistingFiles,
                "conflict_existing_files",
                true,
            ),
            (
                BootstrapState::SubdomainCollision,
                "subdomain_collision",
                true,
            ),
            (BootstrapState::AlreadyDeployed, "already_deployed", true),
            (
                BootstrapState::IdempotencyUnavailable,
                "idempotency_unavailable",
                true,
            ),
            (BootstrapState::Deploying, "deploying", false),
            (BootstrapState::Deployed, "deployed", false),
        ];

        for (state, label, user_decision) in cases {
            assert_eq!(state.as_str(), label);
            assert_eq!(state.is_user_decision(), user_decision, "{label}");
        }
    }
}

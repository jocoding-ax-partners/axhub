use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

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
    AppsCreatePending,
    DeployCreatePending,
    BackendContractMissingDefaults,
    IdempotencyUnavailable,
    AppRegistered,
    Deploying,
    Deployed,
    DependencyInstallRequired,
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
            Self::AppsCreatePending => "apps_create_pending",
            Self::DeployCreatePending => "deploy_create_pending",
            Self::BackendContractMissingDefaults => "backend_contract_missing_defaults",
            Self::IdempotencyUnavailable => "idempotency_unavailable",
            Self::AppRegistered => "app_registered",
            Self::Deploying => "deploying",
            Self::Deployed => "deployed",
            Self::DependencyInstallRequired => "dependency_install_required",
        }
    }

    pub fn is_user_decision(self) -> bool {
        matches!(
            self,
            Self::TemplateRequired
                | Self::ConflictExistingFiles
                | Self::GitInitRequired
                | Self::FirstCommitRequired
                | Self::SubdomainCollision
                | Self::AlreadyDeployed
                | Self::AppsCreatePending
                | Self::DeployCreatePending
                | Self::BackendContractMissingDefaults
                | Self::IdempotencyUnavailable
                | Self::DependencyInstallRequired
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    DependencyInstallRequired,
    DependencyAlreadyInstalled,
    DependencyNotRequired,
}

impl PlanState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DependencyInstallRequired => "dependency_install_required",
            Self::DependencyAlreadyInstalled => "dependency_already_installed",
            Self::DependencyNotRequired => "dependency_not_required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl PackageManager {
    pub fn install_command(self) -> &'static str {
        match self {
            Self::Npm => "npm install",
            Self::Pnpm => "pnpm install",
            Self::Yarn => "yarn install",
            Self::Bun => "bun install",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyPlan {
    pub detected_lockfile: Option<String>,
    pub lockfile_count: u32,
    pub requires_pm_choice: bool,
    pub manager_candidates: Vec<PackageManager>,
    pub recommended_command: Option<String>,
    pub plan_state: PlanState,
    pub package_json_present: bool,
    pub node_modules_present: bool,
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
pub struct NextStep {
    pub id: String,
    pub label: String,
    pub required_for_deploy: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub blocks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_phrase: Option<String>,
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
    pub pending_action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_action_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_steps: Option<Vec<NextStep>>,
}

impl BootstrapOutput {
    fn state(state: BootstrapState) -> Self {
        let user_decision = state.is_user_decision().then(|| state.as_str().to_string());
        Self {
            state,
            next_action: None,
            user_decision,
            command: None,
            pending_action_id: None,
            pending_action_hash: None,
            idempotency_key: None,
            retry_policy: None,
            reason: None,
            next_steps: None,
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

    let mut result = with_bootstrap_phase("plan_next", || {
        plan_next(args.dry_run || !args.auto_chain, args.auto_chain)
    });
    if args.dry_run {
        result.output.next_steps = Some(universal_post_init_next_steps());
    }
    result
}

/// Universal post-init "next safe steps" roadmap rendered by SKILLs after `axhub init`.
/// Source-of-truth lives here so model prose cannot drift (e.g. labelling GitHub
/// connection as `(선택)` when backend rejects deploy with `git_connection_required`).
fn universal_post_init_next_steps() -> Vec<NextStep> {
    vec![
        NextStep {
            id: "app_register".into(),
            label: "앱 등록".into(),
            required_for_deploy: true,
            blocks: vec!["github_connect".into(), "deploy".into()],
            trigger_phrase: Some("axhub 앱 만들어줘".into()),
        },
        NextStep {
            id: "deps_install".into(),
            label: "의존성 설치".into(),
            required_for_deploy: false,
            blocks: vec![],
            trigger_phrase: Some("의존성 설치해".into()),
        },
        NextStep {
            id: "github_connect".into(),
            label: "GitHub 연결".into(),
            required_for_deploy: true,
            blocks: vec!["deploy".into()],
            trigger_phrase: Some("깃허브 연결".into()),
        },
        NextStep {
            id: "env_setup".into(),
            label: "환경 변수".into(),
            required_for_deploy: false,
            blocks: vec![],
            trigger_phrase: Some("환경변수 추가".into()),
        },
        NextStep {
            id: "deploy".into(),
            label: "배포".into(),
            required_for_deploy: true,
            blocks: vec![],
            trigger_phrase: Some("배포해줘".into()),
        },
    ]
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

fn emit_remote_action_planned(event: &str, state: BootstrapState, pending: &PendingAction) {
    emit_bootstrap_marker(
        "remote_action_planned_by_helper",
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

    let commit = git_output(cwd, &["rev-parse", "HEAD"]).unwrap_or_else(|| "HEAD".into());
    let app = state
        .app_slug
        .clone()
        .or_else(|| state.app_id.clone())
        .unwrap_or_else(|| "".into());
    let profile = std::env::var("AXHUB_PROFILE").unwrap_or_default();
    let mut command = vec![
        "axhub".into(),
        "deploy".into(),
        "create".into(),
        "--app".into(),
        app.clone(),
        "--commit".into(),
        commit.clone(),
        "--execute".into(),
        "--json".into(),
    ];
    if !profile.is_empty() {
        command.push("--profile".into());
        command.push(profile.clone());
    }
    plan_remote_action(
        cwd,
        &mut state,
        BootstrapState::DeployCreatePending,
        "deploy_create",
        command,
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
    // Live axhub CLI (v0.17.3) `apps create --from-file` accepts ONLY JSON app
    // definitions — handing it the YAML `apphub.yaml` fails with "expected ident …;
    // JSON app definitions are supported here", which deadlocked first-run deploy.
    // The supported path for a manifest is `apps create --name <n> --slug <slug>`
    // (axhub/src/commands/apps.rs::create_app builds the body from these flags;
    // framework/build come from the manifest at deploy time, not at create). We
    // already parsed the slug from the manifest, so prescribe that command and stop
    // gracefully when the manifest has no name/slug to bind.
    let raw_slug = match manifest.slug.as_deref() {
        Some(value) if !value.is_empty() => value,
        _ => {
            return stop(
                BootstrapState::BackendContractMissingDefaults,
                format!(
                    "{} 에 name/slug 가 없어서 앱을 만들 수 없어요. 'name: <앱이름>' (또는 'slug: <슬러그>') 을 추가하고 다시 배포해요.",
                    manifest.path
                ),
            );
        }
    };
    // The manifest value may be a human display name ("My Cool App") pulled from
    // the `name:` key, but `apps create --slug` needs a backend-valid slug
    // `^[a-z0-9][a-z0-9-]*$`. Emitting the raw name would make
    // the live CLI reject the command and re-deadlock first deploy — the same
    // failure class the `--from-file` fix removed. Slugify so the create command
    // is always valid; stop gracefully when nothing valid can be derived.
    let Some(slug) = slugify(raw_slug) else {
        return stop(
            BootstrapState::BackendContractMissingDefaults,
            format!(
                "{} 의 이름 '{}' 으로 유효한 슬러그를 만들 수 없어요. 영문·숫자 'slug: <슬러그>' 를 추가하고 다시 배포해요.",
                manifest.path, raw_slug
            ),
        );
    };
    let mut state = BootstrapStateFile::new(
        BootstrapState::AppsCreatePending,
        Some(manifest.path.clone()),
    );
    let profile = std::env::var("AXHUB_PROFILE").unwrap_or_default();
    let mut command = vec![
        "axhub".into(),
        "apps".into(),
        "create".into(),
        "--name".into(),
        slug.clone(),
        "--slug".into(),
        slug.clone(),
        "--json".into(),
    ];
    if !profile.is_empty() {
        command.push("--profile".into());
        command.push(profile.clone());
    }
    plan_remote_action(
        &cwd,
        &mut state,
        BootstrapState::AppsCreatePending,
        "apps_create",
        command,
        persist_plan && !plan_only,
    )
}

fn plan_remote_action(
    cwd: &Path,
    state: &mut BootstrapStateFile,
    bootstrap_state: BootstrapState,
    event: &str,
    command: Vec<String>,
    persist_plan: bool,
) -> BootstrapRun {
    let pending_hash = stable_hash(&json!({
        "event": event,
        "command_argv": command,
    }));
    let pending = PendingAction {
        id: format!("{event}:{}", &pending_hash[..16]),
        hash: pending_hash,
        event: event.into(),
        command_argv: command.clone(),
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
    emit_remote_action_planned(event, bootstrap_state, &pending);
    BootstrapRun {
        output: BootstrapOutput::state(bootstrap_state)
            .with_action(event, command)
            .with_pending(&pending)
            .with_retry_policy_for(event),
        exit_code: 0,
    }
}

trait BootstrapOutputExt {
    fn with_retry_policy_for(self, event: &str) -> Self;
}

impl BootstrapOutputExt for BootstrapOutput {
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
    if pending.event == "apps_create" {
        output.retry_policy = Some("no_retry_without_confirmed_idempotency".into());
    }
    emit_remote_action_planned(&pending.event, state.state, pending);
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
    ["axhub.yaml", "apphub.yaml"].into_iter().find_map(|path| {
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

/// Derive a backend-valid slug (`^[a-z0-9][a-z0-9-]*$`) from an arbitrary
/// manifest value. The `name:` fallback in [`parse_manifest_slug`] may yield a
/// human display name like `"My Cool App"`; lowercase it, collapse each run of
/// space/`-`/`_`/`.` into a single hyphen, drop other punctuation and non-ASCII,
/// and trim leading/trailing hyphens. Returns `None` when nothing valid remains
/// (e.g. an all-non-ASCII name like `"내 앱"`), so the caller stops gracefully
/// and asks for an explicit `slug:`. Already-valid slugs (`"paydrop"`) are
/// returned unchanged.
fn slugify(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-' | '_' | '.') && !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let slug = out.trim_matches('-');
    (!slug.is_empty()).then(|| slug.to_string())
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

pub fn build_dependency_plan(cwd: &Path) -> anyhow::Result<DependencyPlan> {
    let package_json_present = cwd.join("package.json").exists();
    let node_modules_present = cwd.join("node_modules").is_dir();

    let lockfile_specs: [(&str, PackageManager); 4] = [
        ("package-lock.json", PackageManager::Npm),
        ("pnpm-lock.yaml", PackageManager::Pnpm),
        ("yarn.lock", PackageManager::Yarn),
        ("bun.lockb", PackageManager::Bun),
    ];

    let mut detected: Vec<(&str, PackageManager)> = Vec::new();
    for (name, pm) in lockfile_specs {
        if cwd.join(name).exists() {
            detected.push((name, pm));
        }
    }

    let lockfile_count = detected.len() as u32;
    let requires_pm_choice = lockfile_count > 1;

    let detected_lockfile = if lockfile_count == 1 {
        Some(detected[0].0.to_string())
    } else {
        None
    };

    let manager_candidates: Vec<PackageManager> = if detected.is_empty() {
        vec![PackageManager::Npm]
    } else {
        detected.iter().map(|(_, pm)| *pm).collect()
    };

    let plan_state = if !package_json_present {
        PlanState::DependencyNotRequired
    } else if node_modules_present {
        PlanState::DependencyAlreadyInstalled
    } else {
        PlanState::DependencyInstallRequired
    };

    let recommended_command =
        if matches!(plan_state, PlanState::DependencyInstallRequired) && !requires_pm_choice {
            Some(manager_candidates[0].install_command().to_string())
        } else {
            None
        };

    Ok(DependencyPlan {
        detected_lockfile,
        lockfile_count,
        requires_pm_choice,
        manager_candidates,
        recommended_command,
        plan_state,
        package_json_present,
        node_modules_present,
    })
}

pub fn cmd_bootstrap_dependency_plan(args: &[String]) -> anyhow::Result<i32> {
    let mut json_output = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json_output = true,
            other => {
                eprintln!("axhub-helpers bootstrap dependency-plan: unknown option \"{other}\"");
                return Ok(64);
            }
        }
    }
    let cwd = std::env::current_dir()?;
    let plan = build_dependency_plan(&cwd)?;
    let exit_code = if plan.requires_pm_choice { 65 } else { 0 };
    if json_output {
        println!("{}", serde_json::to_string(&plan)?);
    } else {
        println!("plan_state: {}", plan.plan_state.as_str());
        if let Some(cmd) = plan.recommended_command.as_ref() {
            println!("recommended_command: {cmd}");
        }
        if plan.requires_pm_choice {
            let candidates: Vec<&str> = plan
                .manager_candidates
                .iter()
                .map(|pm| pm.install_command())
                .collect();
            println!(
                "multiple lockfiles detected; choose one: {}",
                candidates.join(", ")
            );
        }
    }
    Ok(exit_code)
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

    #[test]
    fn universal_next_steps_match_init_skill_contract() {
        let steps = universal_post_init_next_steps();
        assert_eq!(steps.len(), 5, "init flow has 5 universal next steps");

        let ids: Vec<&str> = steps.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "app_register",
                "deps_install",
                "github_connect",
                "env_setup",
                "deploy"
            ]
        );

        let required: Vec<&str> = steps
            .iter()
            .filter(|s| s.required_for_deploy)
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(
            required,
            vec!["app_register", "github_connect", "deploy"],
            "GitHub connect must be required_for_deploy=true (backend rejects with HTTP 422 git_connection_required)"
        );

        let github = steps.iter().find(|s| s.id == "github_connect").unwrap();
        assert!(
            github.blocks.iter().any(|b| b == "deploy"),
            "github_connect must declare it blocks deploy"
        );
    }

    #[test]
    fn next_step_serialization_matches_init_skill_render_contract() {
        let step = NextStep {
            id: "github_connect".into(),
            label: "GitHub 연결".into(),
            required_for_deploy: true,
            blocks: vec!["deploy".into()],
            trigger_phrase: Some("깃허브 연결".into()),
        };
        let serialized = serde_json::to_value(&step).unwrap();
        assert_eq!(serialized["id"], "github_connect");
        assert_eq!(serialized["label"], "GitHub 연결");
        assert_eq!(serialized["required_for_deploy"], true);
        assert_eq!(serialized["blocks"][0], "deploy");
        assert_eq!(serialized["trigger_phrase"], "깃허브 연결");
    }

    #[test]
    fn next_step_omits_empty_blocks_and_none_trigger_phrase() {
        // PR95 review finding: verify skip_serializing_if behavior on empty blocks[]
        // and None trigger_phrase. Init SKILL Step 6 render contract assumes both
        // optional fields disappear when empty so the rendered example block stays clean.
        let leaf_step = NextStep {
            id: "deploy".into(),
            label: "배포".into(),
            required_for_deploy: true,
            blocks: vec![],
            trigger_phrase: None,
        };
        let serialized = serde_json::to_value(&leaf_step).unwrap();
        let obj = serialized.as_object().unwrap();
        assert!(
            !obj.contains_key("blocks"),
            "empty blocks[] must be omitted via skip_serializing_if (Vec::is_empty)"
        );
        assert!(
            !obj.contains_key("trigger_phrase"),
            "None trigger_phrase must be omitted via skip_serializing_if (Option::is_none)"
        );
        assert_eq!(serialized["id"], "deploy");
        assert_eq!(serialized["required_for_deploy"], true);
    }

    #[test]
    fn slugify_derives_valid_slug_from_display_name() {
        // The bug: a manifest with `name: My Cool App` and no explicit slug fed
        // the raw name straight into `apps create --slug`, which the live CLI
        // rejects (`^[a-z0-9][a-z0-9-]*$`), re-deadlocking first deploy.
        assert_eq!(slugify("My Cool App").as_deref(), Some("my-cool-app"));
        assert_eq!(slugify("  Spaced  Out  ").as_deref(), Some("spaced-out"));
        assert_eq!(
            slugify("Under_score.dot-dash").as_deref(),
            Some("under-score-dot-dash")
        );
        assert_eq!(slugify("Trailing---").as_deref(), Some("trailing"));
        assert_eq!(slugify("App!!! v2").as_deref(), Some("app-v2"));
    }

    #[test]
    fn slugify_passes_through_already_valid_slug() {
        // Idempotent on canonical slugs so the proven `paydrop` path is unchanged.
        assert_eq!(slugify("paydrop").as_deref(), Some("paydrop"));
        assert_eq!(slugify("my-cool-app").as_deref(), Some("my-cool-app"));
        assert_eq!(slugify("app123").as_deref(), Some("app123"));
    }

    #[test]
    fn slugify_emitted_slug_matches_backend_regex() {
        let re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*$").unwrap();
        for name in ["My Cool App", "123 Start", "a.b_c-d", "MIXED Case 9"] {
            let slug = slugify(name).expect("should derive a slug");
            assert!(
                re.is_match(&slug),
                "slug {slug:?} from {name:?} must match backend regex"
            );
        }
    }

    #[test]
    fn slugify_returns_none_when_nothing_valid_remains() {
        // Non-ASCII (e.g. a Korean name) or punctuation-only input yields no valid
        // slug → caller stops gracefully and asks for an explicit `slug:`.
        assert_eq!(slugify("내 앱"), None);
        assert_eq!(slugify("!!!"), None);
        assert_eq!(slugify("   "), None);
        assert_eq!(slugify(""), None);
    }

    #[test]
    fn plan_apps_create_slugifies_display_name_into_valid_command() {
        // End-to-end at the planner level: a display-name manifest must emit a
        // valid backend slug in the command.
        let manifest = ManifestInfo {
            path: "axhub.yaml".into(),
            slug: Some("My Cool App".into()),
        };
        let run = plan_apps_create(&manifest, true, false);
        let cmd = run
            .output
            .command
            .as_ref()
            .expect("apps_create plan must carry a command");
        let slug_idx = cmd
            .iter()
            .position(|a| a == "--slug")
            .expect("--slug present");
        assert_eq!(
            cmd[slug_idx + 1],
            "my-cool-app",
            "emitted slug must be valid"
        );
        assert!(run.output.pending_action_id.is_some());
        assert!(run.output.pending_action_hash.is_some());
    }
}

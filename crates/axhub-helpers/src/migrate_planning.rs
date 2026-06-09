use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MIGRATE_PLANNING_PREVIEW_SCHEMA_VERSION: &str = "migrate-planning-preview/v1";
pub const MIGRATE_SPEC_LATEST_SCHEMA_VERSION: &str = "axhub/migrate-spec-latest/v1";
pub const MIGRATE_SPEC_META_SCHEMA_VERSION: &str = "axhub/migrate-spec/v1";
pub const MIGRATE_PLAN_RUN_SCHEMA_VERSION: &str = "axhub/migrate-plan-run/v1";
pub const MIGRATE_PLAN_APPROVAL_SCHEMA_VERSION: &str = "axhub/migrate-plan-approval/v1";
pub const MIGRATE_PLAN_WAVES_SCHEMA_VERSION: &str = "axhub/migrate-plan-waves/v1";
pub const MIGRATE_PLAN_WAVE_SCHEMA_VERSION: &str = "axhub/migrate-plan-wave/v1";
pub const WORKSPACE_MARKER_SCHEMA_VERSION: &str = "axhub/workspace-marker/v1";
pub const WORKSPACE_MARKER_FILENAME: &str = ".axhub-workspace";

pub const FULL_CONSENSUS_STAGE_ORDER: [&str; 5] =
    ["discover", "planner", "architect", "critic", "reviewer"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningMode {
    Simple,
    SpecOnly,
    FullConsensus,
}

impl PlanningMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::SpecOnly => "spec_only",
            Self::FullConsensus => "full_consensus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationReason {
    None,
    LowConfidence,
    HardStop,
    Complexity,
    UserRequested,
}

impl EscalationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LowConfidence => "low_confidence",
            Self::HardStop => "hard_stop",
            Self::Complexity => "complexity",
            Self::UserRequested => "user_requested",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Draft,
    Running,
    PendingApproval,
    Approved,
    NeedsRevision,
    Rejected,
    Aborted,
    Superseded,
}

impl RunState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Running)
                | (Self::Draft, Self::PendingApproval)
                | (Self::Draft, Self::Aborted)
                | (Self::Running, Self::PendingApproval)
                | (Self::Running, Self::NeedsRevision)
                | (Self::Running, Self::Aborted)
                | (Self::PendingApproval, Self::Approved)
                | (Self::PendingApproval, Self::Rejected)
                | (Self::PendingApproval, Self::NeedsRevision)
                | (Self::PendingApproval, Self::Aborted)
                | (Self::NeedsRevision, Self::Draft)
                | (Self::NeedsRevision, Self::Running)
                | (Self::NeedsRevision, Self::Aborted)
                | (Self::Approved, Self::Superseded)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecState {
    Draft,
    PendingApproval,
    Approved,
    NeedsRevision,
    Rejected,
    Superseded,
}

impl SpecState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::PendingApproval)
                | (Self::Draft, Self::Rejected)
                | (Self::PendingApproval, Self::Approved)
                | (Self::PendingApproval, Self::Rejected)
                | (Self::PendingApproval, Self::NeedsRevision)
                | (Self::NeedsRevision, Self::Draft)
                | (Self::NeedsRevision, Self::Rejected)
                | (Self::Approved, Self::Superseded)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    PendingApproval,
    Approved,
    Rejected,
    NeedsRevision,
}

impl ApprovalState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::PendingApproval, Self::Approved)
                | (Self::PendingApproval, Self::Rejected)
                | (Self::PendingApproval, Self::NeedsRevision)
                | (Self::NeedsRevision, Self::PendingApproval)
                | (Self::NeedsRevision, Self::Rejected)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveState {
    Planned,
    Running,
    Complete,
    NeedsRevision,
    Blocked,
    Aborted,
}

impl WaveState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Planned, Self::Running)
                | (Self::Planned, Self::Blocked)
                | (Self::Planned, Self::Aborted)
                | (Self::Running, Self::Complete)
                | (Self::Running, Self::NeedsRevision)
                | (Self::Running, Self::Blocked)
                | (Self::Running, Self::Aborted)
                | (Self::Blocked, Self::Planned)
                | (Self::Blocked, Self::Aborted)
                | (Self::NeedsRevision, Self::Planned)
                | (Self::NeedsRevision, Self::Aborted)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceMarker {
    pub schema_version: String,
    pub shared_planning: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConflict {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScope {
    #[serde(rename = "type")]
    pub scope_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<WorkspaceConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParallelismPolicy {
    pub enabled: bool,
    pub scope: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WavePolicy {
    pub mode: String,
    pub independence_required: bool,
    pub multi_app_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratePlanRunRecord {
    pub schema_version: String,
    pub run_id: String,
    pub app_key: String,
    pub app_path: String,
    pub repo_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_app_keys: Option<Vec<String>>,
    pub mode: PlanningMode,
    pub stage_order: Vec<String>,
    pub state: RunState,
    pub escalation_reason: EscalationReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hard_stop_reasons: Vec<String>,
    pub workspace_scope: WorkspaceScope,
    pub parallelism: ParallelismPolicy,
    pub wave_policy: WavePolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wave_index_path: Option<String>,
    pub repo_fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
}

impl MigratePlanRunRecord {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != MIGRATE_PLAN_RUN_SCHEMA_VERSION {
            bail!("migrate planning run: unknown schema version");
        }
        if self.app_key.trim().is_empty() {
            bail!("migrate planning run: app_key is required");
        }
        if let Some(keys) = self.owned_app_keys.as_ref() {
            if keys.len() != 1 || keys.first().map(String::as_str) != Some(self.app_key.as_str()) {
                bail!("migrate planning run: owned_app_keys must contain exactly the run app_key");
            }
        }
        if self.wave_policy.multi_app_allowed {
            bail!("migrate planning run: multi_app_allowed must remain false in v1");
        }
        match self.mode {
            PlanningMode::Simple => {
                if self.state != RunState::Draft && self.state != RunState::Running {
                    bail!(
                        "migrate planning run: simple mode must not be sealed as an approval run"
                    );
                }
            }
            PlanningMode::SpecOnly => {
                if !self.stage_order.is_empty() {
                    bail!("migrate planning run: spec_only must keep empty stage_order");
                }
                if self.parallelism.enabled {
                    bail!("migrate planning run: spec_only must remain serial");
                }
                if self.wave_index_path.is_some() {
                    bail!("migrate planning run: spec_only must not declare wave metadata");
                }
            }
            PlanningMode::FullConsensus => {
                if self.stage_order
                    != FULL_CONSENSUS_STAGE_ORDER
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                {
                    bail!("migrate planning run: full_consensus stage order mismatch");
                }
                if self.parallelism.enabled && self.wave_index_path.is_none() {
                    bail!("migrate planning run: parallel full_consensus requires wave_index_path");
                }
            }
        }
        Ok(())
    }

    pub fn assert_repo_fingerprint_matches(&self, repo_root: &Path) -> Result<()> {
        let canonical = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());
        let actual = sha256_hex(&canonical.display().to_string());
        if self.repo_fingerprint != actual {
            bail!("migrate planning run: repo fingerprint mismatch");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratePlanApprovalRecord {
    pub schema_version: String,
    pub run_id: String,
    pub app_key: String,
    pub state: ApprovalState,
    pub required_before_execution: bool,
    pub target_spec_id: String,
    pub target_spec_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_stage_artifacts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adr_sha256: Option<String>,
    pub requested_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    pub approval_prompt_sha256: String,
}

impl MigratePlanApprovalRecord {
    pub fn validate(&self, mode: PlanningMode) -> Result<()> {
        if self.schema_version != MIGRATE_PLAN_APPROVAL_SCHEMA_VERSION {
            bail!("migrate plan approval: unknown schema version");
        }
        if !self.required_before_execution {
            bail!("migrate plan approval: execution must stay approval-gated");
        }
        match mode {
            PlanningMode::SpecOnly => {
                if !self.approved_stage_artifacts.is_empty() {
                    bail!("migrate plan approval: spec_only must not include approved stage artifacts");
                }
                if self.adr_sha256.is_some() {
                    bail!("migrate plan approval: spec_only must not include adr sha");
                }
            }
            PlanningMode::FullConsensus => {
                if self.state == ApprovalState::PendingApproval
                    && self.approved_stage_artifacts.is_empty()
                {
                    bail!("migrate plan approval: full_consensus pending approval requires stage sha list");
                }
                if self.state == ApprovalState::PendingApproval && self.adr_sha256.is_none() {
                    bail!(
                        "migrate plan approval: full_consensus pending approval requires adr sha"
                    );
                }
            }
            PlanningMode::Simple => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratePlanWaveIndex {
    pub schema_version: String,
    pub run_id: String,
    pub app_key: String,
    pub enabled: bool,
    pub policy: String,
    pub multi_app_allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub waves: Vec<String>,
    pub dependency_graph_sha256: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratePlanWaveRecord {
    pub schema_version: String,
    pub run_id: String,
    pub app_key: String,
    pub wave_id: String,
    pub wave_n: u32,
    pub stage_scope: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    pub dependency_graph: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_list: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_targets: Vec<String>,
    pub state: WaveState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub independence_proof: Vec<String>,
    pub conflict_policy: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanningPreview {
    pub schema_version: String,
    pub app_key: String,
    pub mode: PlanningMode,
    pub escalation_reason: EscalationReason,
    pub selection_locked: bool,
    pub candidate_confidence: Option<String>,
    pub pending_approval_required: bool,
    pub workspace_scope: WorkspaceScope,
    pub repo_fingerprint: String,
    pub path_templates: PlanningPathTemplates,
    pub parallelism: ParallelismPolicy,
    pub wave_policy: WavePolicy,
    pub stage_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanningPathTemplates {
    pub planning_root: String,
    pub spec_latest_path: String,
    pub spec_markdown_path_template: String,
    pub spec_meta_path_template: String,
    pub run_dir_template: String,
    pub run_json_path_template: String,
    pub approval_json_path_template: String,
    pub receipts_path_template: String,
    pub latest_run_path: String,
    pub stage_markdown_path_template: String,
    pub stage_meta_path_template: String,
    pub adr_path_template: String,
    pub wave_index_path_template: String,
    pub wave_record_path_template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlanningRoot {
    pub repo_root: PathBuf,
    pub planning_root: PathBuf,
    pub workspace_scope: WorkspaceScope,
    pub repo_fingerprint: String,
}

pub fn build_planning_preview(
    repo_root: &Path,
    selected_app_path: Option<&str>,
    candidate_count: usize,
    selection_locked: bool,
    candidate_confidence: Option<f64>,
    hard_stop_reasons: &[String],
) -> Result<Option<PlanningPreview>> {
    let Some(selected_app_path) = selected_app_path else {
        return Ok(None);
    };
    let resolved = resolve_planning_root(repo_root)?;
    let app_key = build_app_key(&resolved.repo_root, selected_app_path);
    let mode = determine_mode(
        selection_locked,
        candidate_count,
        candidate_confidence,
        hard_stop_reasons,
    );
    let escalation_reason = determine_escalation_reason(
        selection_locked,
        candidate_count,
        candidate_confidence,
        hard_stop_reasons,
    );
    let path_templates = build_path_templates(&resolved.planning_root, &app_key);
    let parallelism = match mode {
        PlanningMode::FullConsensus => ParallelismPolicy {
            enabled: true,
            scope: "same_app_only".to_string(),
            reason: "conditional_parallel_after_independence_proof".to_string(),
            fallback_reason: Some("fallback_to_serial_on_conflict_or_uncertainty".to_string()),
        },
        PlanningMode::Simple | PlanningMode::SpecOnly => ParallelismPolicy {
            enabled: false,
            scope: "serial".to_string(),
            reason: if mode == PlanningMode::SpecOnly {
                "spec_only_serial_required".to_string()
            } else {
                "simple_flow_preserved".to_string()
            },
            fallback_reason: None,
        },
    };
    let stage_order = match mode {
        PlanningMode::FullConsensus => FULL_CONSENSUS_STAGE_ORDER
            .iter()
            .map(|s| s.to_string())
            .collect(),
        PlanningMode::Simple | PlanningMode::SpecOnly => vec![],
    };
    Ok(Some(PlanningPreview {
        schema_version: MIGRATE_PLANNING_PREVIEW_SCHEMA_VERSION.to_string(),
        app_key,
        mode,
        escalation_reason,
        selection_locked,
        candidate_confidence: candidate_confidence.map(|v| format!("{v:.2}")),
        pending_approval_required: mode != PlanningMode::Simple,
        workspace_scope: resolved.workspace_scope,
        repo_fingerprint: resolved.repo_fingerprint,
        path_templates,
        parallelism,
        wave_policy: WavePolicy {
            mode: if mode == PlanningMode::FullConsensus {
                "conditional_parallel".to_string()
            } else {
                "serial".to_string()
            },
            independence_required: true,
            multi_app_allowed: false,
        },
        stage_order,
    }))
}

pub fn resolve_planning_root(repo_root: &Path) -> Result<ResolvedPlanningRoot> {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let default_root = repo_root.join(".axhub");
    let repo_fingerprint = sha256_hex(&repo_root.display().to_string());
    let marker = find_workspace_marker(&repo_root)?;
    let workspace_scope = match marker {
        Some(FoundMarker::Valid {
            path,
            root,
            shared_planning,
            marker_sha256,
        }) => {
            if shared_planning {
                WorkspaceScope {
                    scope_type: "workspace".to_string(),
                    marker_path: Some(path.display().to_string()),
                    workspace_root: Some(root.display().to_string()),
                    marker_sha256: Some(marker_sha256),
                    conflict: None,
                }
            } else {
                WorkspaceScope {
                    scope_type: "repo".to_string(),
                    marker_path: Some(path.display().to_string()),
                    workspace_root: Some(root.display().to_string()),
                    marker_sha256: Some(marker_sha256),
                    conflict: None,
                }
            }
        }
        Some(FoundMarker::Invalid { path, detail }) => WorkspaceScope {
            scope_type: "repo".to_string(),
            marker_path: Some(path.display().to_string()),
            workspace_root: Some(path.parent().unwrap_or(&repo_root).display().to_string()),
            marker_sha256: None,
            conflict: Some(WorkspaceConflict {
                code: "marker_invalid_fail_closed".to_string(),
                detail,
            }),
        },
        None => WorkspaceScope {
            scope_type: "repo".to_string(),
            marker_path: None,
            workspace_root: Some(repo_root.display().to_string()),
            marker_sha256: None,
            conflict: None,
        },
    };
    let planning_root = if workspace_scope.scope_type == "workspace" {
        PathBuf::from(
            workspace_scope
                .workspace_root
                .as_deref()
                .unwrap_or(repo_root.to_str().unwrap_or(".")),
        )
        .join(".axhub")
    } else {
        default_root
    };
    Ok(ResolvedPlanningRoot {
        repo_root,
        planning_root,
        workspace_scope,
        repo_fingerprint,
    })
}

pub fn build_app_key(repo_root: &Path, app_path: &str) -> String {
    let normalized_app_path = normalize_app_path(app_path);
    let slug = slugify_component(&normalized_app_path);
    let identity = format!("{}::{}", repo_root.display(), normalized_app_path);
    let hash = short_sha256(&identity, 8);
    format!("{slug}-{hash}")
}

pub fn validate_wave_plan(
    index: &MigratePlanWaveIndex,
    waves: &[MigratePlanWaveRecord],
) -> Result<()> {
    if index.schema_version != MIGRATE_PLAN_WAVES_SCHEMA_VERSION {
        bail!("migrate wave index: unknown schema version");
    }
    if !index.enabled {
        bail!("migrate wave index: disabled index is invalid for persisted waves");
    }
    if index.multi_app_allowed {
        bail!("migrate wave index: multi_app_allowed must remain false");
    }
    let wave_ids: BTreeSet<_> = waves.iter().map(|wave| wave.wave_id.as_str()).collect();
    if wave_ids.len() != waves.len() {
        bail!("migrate wave index: duplicate wave_id");
    }
    for wave in waves {
        if wave.schema_version != MIGRATE_PLAN_WAVE_SCHEMA_VERSION {
            bail!("migrate wave: unknown schema version");
        }
        if wave.app_key != index.app_key {
            bail!("migrate wave: participant app_key must match run app_key");
        }
        if wave.conflict_policy != "fallback_to_serial" {
            bail!("migrate wave: conflict policy must remain fallback_to_serial");
        }
        let mut seen_targets = BTreeSet::new();
        for target in &wave.write_targets {
            if !seen_targets.insert(target) {
                bail!("migrate wave: duplicate write target inside one wave");
            }
        }
        if wave.independence_proof.is_empty() {
            bail!("migrate wave: independence proof is required");
        }
        for participant in &wave.participants {
            if participant != &index.app_key {
                bail!("migrate wave: all participants must share the run app_key");
            }
        }
    }
    let graph = waves
        .iter()
        .map(|wave| (wave.wave_id.clone(), wave.depends_on.clone()))
        .collect::<BTreeMap<_, _>>();
    if has_cycle(&graph) {
        bail!("migrate wave index: dependency cycles are not allowed");
    }
    let mut by_target = BTreeMap::<&str, &str>::new();
    for wave in waves {
        for target in &wave.write_targets {
            if let Some(existing) = by_target.insert(target, &wave.wave_id) {
                bail!("migrate wave index: concurrently runnable waves cannot share write target ({existing} vs {})", wave.wave_id);
            }
        }
    }
    Ok(())
}

fn determine_mode(
    selection_locked: bool,
    candidate_count: usize,
    candidate_confidence: Option<f64>,
    hard_stop_reasons: &[String],
) -> PlanningMode {
    if !hard_stop_reasons.is_empty() {
        PlanningMode::FullConsensus
    } else if !selection_locked || candidate_count > 1 || candidate_confidence.unwrap_or(0.0) < 0.80
    {
        PlanningMode::SpecOnly
    } else {
        PlanningMode::Simple
    }
}

fn determine_escalation_reason(
    selection_locked: bool,
    candidate_count: usize,
    candidate_confidence: Option<f64>,
    hard_stop_reasons: &[String],
) -> EscalationReason {
    if !hard_stop_reasons.is_empty() {
        EscalationReason::HardStop
    } else if !selection_locked || candidate_count > 1 {
        EscalationReason::Complexity
    } else if candidate_confidence.unwrap_or(0.0) < 0.80 {
        EscalationReason::LowConfidence
    } else {
        EscalationReason::None
    }
}

fn build_path_templates(planning_root: &Path, app_key: &str) -> PlanningPathTemplates {
    let spec_root = planning_root.join("spec").join("apps").join(app_key);
    let plan_root = planning_root.join("plan");
    let run_dir = plan_root.join("runs").join("<run_id>");
    let spec_dir = spec_root.join("specs");
    PlanningPathTemplates {
        planning_root: planning_root.display().to_string(),
        spec_latest_path: spec_root.join("latest.json").display().to_string(),
        spec_markdown_path_template: spec_dir.join("<spec_id>.md").display().to_string(),
        spec_meta_path_template: spec_dir.join("<spec_id>.meta.json").display().to_string(),
        run_dir_template: run_dir.display().to_string(),
        run_json_path_template: run_dir.join("run.json").display().to_string(),
        approval_json_path_template: run_dir.join("approval.json").display().to_string(),
        receipts_path_template: run_dir.join("receipts.jsonl").display().to_string(),
        latest_run_path: plan_root
            .join("apps")
            .join(app_key)
            .join("latest-run.json")
            .display()
            .to_string(),
        stage_markdown_path_template: run_dir
            .join("stages")
            .join("<nn>-<stage>.md")
            .display()
            .to_string(),
        stage_meta_path_template: run_dir
            .join("stages")
            .join("<nn>-<stage>.meta.json")
            .display()
            .to_string(),
        adr_path_template: run_dir.join("adr.md").display().to_string(),
        wave_index_path_template: run_dir
            .join("waves")
            .join("waves.json")
            .display()
            .to_string(),
        wave_record_path_template: run_dir
            .join("waves")
            .join("<wave_id>.json")
            .display()
            .to_string(),
    }
}

#[derive(Debug)]
enum FoundMarker {
    Valid {
        path: PathBuf,
        root: PathBuf,
        shared_planning: bool,
        marker_sha256: String,
    },
    Invalid {
        path: PathBuf,
        detail: String,
    },
}

fn find_workspace_marker(repo_root: &Path) -> Result<Option<FoundMarker>> {
    let home_boundary = home_boundary();
    let mut current = Some(repo_root);
    while let Some(dir) = current {
        let candidate = dir.join(WORKSPACE_MARKER_FILENAME);
        if candidate.exists() {
            let raw = match fs::read_to_string(&candidate) {
                Ok(raw) => raw,
                Err(err) => {
                    return Ok(Some(FoundMarker::Invalid {
                        path: candidate,
                        detail: format!("marker_unreadable:{err}"),
                    }))
                }
            };
            let parsed: WorkspaceMarker = match serde_json::from_str(&raw) {
                Ok(parsed) => parsed,
                Err(err) => {
                    return Ok(Some(FoundMarker::Invalid {
                        path: candidate,
                        detail: format!("marker_malformed:{err}"),
                    }))
                }
            };
            if parsed.schema_version != WORKSPACE_MARKER_SCHEMA_VERSION {
                return Ok(Some(FoundMarker::Invalid {
                    path: candidate,
                    detail: format!("marker_schema_unsupported:{}", parsed.schema_version),
                }));
            }
            return Ok(Some(FoundMarker::Valid {
                marker_sha256: sha256_hex(&raw),
                path: candidate,
                root: dir.to_path_buf(),
                shared_planning: parsed.shared_planning,
            }));
        }
        if home_boundary.as_deref() == Some(dir) {
            break;
        }
        current = dir.parent();
    }
    Ok(None)
}

fn home_boundary() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

fn normalize_app_path(app_path: &str) -> String {
    let trimmed = app_path.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed == "." {
        ".".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn slugify_component(app_path: &str) -> String {
    let raw = normalize_app_path(app_path);
    let source = if raw == "." { "root".to_string() } else { raw };
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in source.chars() {
        let mapped = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            _ => None,
        };
        if let Some(ch) = mapped {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let collapsed = out.trim_matches('-');
    if collapsed.is_empty() {
        "app".to_string()
    } else {
        collapsed.to_string()
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn short_sha256(input: &str, len: usize) -> String {
    let full = sha256_hex(input);
    full[..len.min(full.len())].to_string()
}

fn has_cycle(graph: &BTreeMap<String, Vec<String>>) -> bool {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visited.contains(node) {
            return false;
        }
        if !visiting.insert(node.to_string()) {
            return true;
        }
        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if visit(dep, graph, visiting, visited) {
                    return true;
                }
            }
        }
        visiting.remove(node);
        visited.insert(node.to_string());
        false
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    graph
        .keys()
        .any(|node| visit(node, graph, &mut visiting, &mut visited))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview_for(
        root: &Path,
        app_path: &str,
        candidate_count: usize,
        selection_locked: bool,
        confidence: Option<f64>,
        hard_stop_reasons: &[&str],
    ) -> PlanningPreview {
        build_planning_preview(
            root,
            Some(app_path),
            candidate_count,
            selection_locked,
            confidence,
            &hard_stop_reasons
                .iter()
                .map(|reason| reason.to_string())
                .collect::<Vec<_>>(),
        )
        .unwrap()
        .unwrap()
    }

    #[test]
    fn planning_preview_defaults_to_simple_for_high_confidence_single_app() {
        let temp = tempfile::tempdir().unwrap();
        let preview = preview_for(temp.path(), ".", 1, true, Some(0.92), &[]);
        assert_eq!(preview.mode, PlanningMode::Simple);
        assert_eq!(preview.escalation_reason, EscalationReason::None);
        assert_eq!(preview.workspace_scope.scope_type, "repo");
        assert!(
            preview
                .path_templates
                .spec_latest_path
                .ends_with(".axhub/spec/apps/root-".to_string().as_str())
                == false
        );
    }

    #[test]
    fn planning_preview_escalates_low_confidence_to_spec_only() {
        let temp = tempfile::tempdir().unwrap();
        let preview = preview_for(temp.path(), ".", 1, true, Some(0.62), &[]);
        assert_eq!(preview.mode, PlanningMode::SpecOnly);
        assert_eq!(preview.escalation_reason, EscalationReason::LowConfidence);
        assert!(!preview.parallelism.enabled);
        assert!(preview.stage_order.is_empty());
    }

    #[test]
    fn planning_preview_escalates_hard_stop_to_full_consensus() {
        let temp = tempfile::tempdir().unwrap();
        let preview = preview_for(temp.path(), "apps/web", 2, true, Some(0.88), &["wide diff"]);
        assert_eq!(preview.mode, PlanningMode::FullConsensus);
        assert_eq!(preview.escalation_reason, EscalationReason::HardStop);
        assert_eq!(
            preview.stage_order,
            FULL_CONSENSUS_STAGE_ORDER
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
        assert!(preview.parallelism.enabled);
    }

    #[test]
    fn build_app_key_sanitizes_path_and_hashes_identity() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let key = build_app_key(root, "apps/web");
        assert!(key.starts_with("apps-web-"));
        assert_eq!(key.len(), "apps-web-".len() + 8);
        assert_ne!(key, build_app_key(root, "apps/api"));
    }

    #[test]
    fn resolve_planning_root_uses_workspace_marker_opt_in() {
        let workspace = tempfile::tempdir().unwrap();
        let repo = workspace.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            workspace.path().join(WORKSPACE_MARKER_FILENAME),
            format!(
                "{{\"schema_version\":\"{}\",\"shared_planning\":true}}",
                WORKSPACE_MARKER_SCHEMA_VERSION
            ),
        )
        .unwrap();
        let resolved = resolve_planning_root(&repo).unwrap();
        assert_eq!(resolved.workspace_scope.scope_type, "workspace");
        let canonical_workspace = std::fs::canonicalize(workspace.path()).unwrap();
        assert_eq!(resolved.planning_root, canonical_workspace.join(".axhub"));
    }

    #[test]
    fn malformed_nearest_marker_fails_closed_without_falling_through() {
        let workspace = tempfile::tempdir().unwrap();
        let repo = workspace.path().join("parent/repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            workspace
                .path()
                .join("parent")
                .join(WORKSPACE_MARKER_FILENAME),
            "{bad json",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join(WORKSPACE_MARKER_FILENAME),
            format!(
                "{{\"schema_version\":\"{}\",\"shared_planning\":true}}",
                WORKSPACE_MARKER_SCHEMA_VERSION
            ),
        )
        .unwrap();
        let resolved = resolve_planning_root(&repo).unwrap();
        assert_eq!(resolved.workspace_scope.scope_type, "repo");
        assert!(resolved.workspace_scope.conflict.is_some());
        let canonical_repo = std::fs::canonicalize(&repo).unwrap();
        assert_eq!(resolved.planning_root, canonical_repo.join(".axhub"));
    }

    #[test]
    fn run_record_rejects_multi_app_and_spec_only_parallelism() {
        let record = MigratePlanRunRecord {
            schema_version: MIGRATE_PLAN_RUN_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            app_path: ".".to_string(),
            repo_root: "/repo".to_string(),
            remote_repo: None,
            r#ref: None,
            owned_app_keys: Some(vec![
                "root-12345678".to_string(),
                "other-87654321".to_string(),
            ]),
            mode: PlanningMode::SpecOnly,
            stage_order: vec!["discover".to_string()],
            state: RunState::Draft,
            escalation_reason: EscalationReason::LowConfidence,
            confidence: Some("0.61".to_string()),
            hard_stop_reasons: vec![],
            workspace_scope: WorkspaceScope {
                scope_type: "repo".to_string(),
                marker_path: None,
                workspace_root: Some("/repo".to_string()),
                marker_sha256: None,
                conflict: None,
            },
            parallelism: ParallelismPolicy {
                enabled: true,
                scope: "serial".to_string(),
                reason: "wrong".to_string(),
                fallback_reason: None,
            },
            wave_policy: WavePolicy {
                mode: "serial".to_string(),
                independence_required: true,
                multi_app_allowed: false,
            },
            wave_index_path: Some("/repo/.axhub/plan/runs/run-1/waves/waves.json".to_string()),
            repo_fingerprint: "abc".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let error = record.validate().unwrap_err().to_string();
        assert!(error.contains("owned_app_keys") || error.contains("spec_only"));
    }

    #[test]
    fn full_consensus_run_requires_exact_stage_order() {
        let record = MigratePlanRunRecord {
            schema_version: MIGRATE_PLAN_RUN_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            app_path: ".".to_string(),
            repo_root: "/repo".to_string(),
            remote_repo: None,
            r#ref: None,
            owned_app_keys: Some(vec!["root-12345678".to_string()]),
            mode: PlanningMode::FullConsensus,
            stage_order: vec!["planner".to_string(), "discover".to_string()],
            state: RunState::Running,
            escalation_reason: EscalationReason::HardStop,
            confidence: Some("0.91".to_string()),
            hard_stop_reasons: vec!["wide diff".to_string()],
            workspace_scope: WorkspaceScope {
                scope_type: "repo".to_string(),
                marker_path: None,
                workspace_root: Some("/repo".to_string()),
                marker_sha256: None,
                conflict: None,
            },
            parallelism: ParallelismPolicy {
                enabled: false,
                scope: "serial".to_string(),
                reason: "fallback".to_string(),
                fallback_reason: None,
            },
            wave_policy: WavePolicy {
                mode: "conditional_parallel".to_string(),
                independence_required: true,
                multi_app_allowed: false,
            },
            wave_index_path: None,
            repo_fingerprint: "abc".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert!(record
            .validate()
            .unwrap_err()
            .to_string()
            .contains("stage order"));
    }

    #[test]
    fn run_record_rejects_repo_fingerprint_collision() {
        let repo = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let app_key = build_app_key(repo.path(), ".");
        let record = MigratePlanRunRecord {
            schema_version: MIGRATE_PLAN_RUN_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: app_key.clone(),
            app_path: ".".to_string(),
            repo_root: repo.path().display().to_string(),
            remote_repo: None,
            r#ref: None,
            owned_app_keys: Some(vec![app_key]),
            mode: PlanningMode::SpecOnly,
            stage_order: vec![],
            state: RunState::Draft,
            escalation_reason: EscalationReason::LowConfidence,
            confidence: Some("0.61".to_string()),
            hard_stop_reasons: vec![],
            workspace_scope: WorkspaceScope {
                scope_type: "repo".to_string(),
                marker_path: None,
                workspace_root: Some(repo.path().display().to_string()),
                marker_sha256: None,
                conflict: None,
            },
            parallelism: ParallelismPolicy {
                enabled: false,
                scope: "serial".to_string(),
                reason: "spec_only_serial_required".to_string(),
                fallback_reason: None,
            },
            wave_policy: WavePolicy {
                mode: "serial".to_string(),
                independence_required: true,
                multi_app_allowed: false,
            },
            wave_index_path: None,
            repo_fingerprint: sha256_hex(
                &repo.path().canonicalize().unwrap().display().to_string(),
            ),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert!(record
            .assert_repo_fingerprint_matches(other.path())
            .is_err());
    }
    #[test]
    fn transition_matrices_match_plan_contract() {
        assert!(RunState::Draft.can_transition_to(RunState::Running));
        assert!(!RunState::Approved.can_transition_to(RunState::Running));
        assert!(SpecState::PendingApproval.can_transition_to(SpecState::Approved));
        assert!(!SpecState::Rejected.can_transition_to(SpecState::Draft));
        assert!(ApprovalState::NeedsRevision.can_transition_to(ApprovalState::PendingApproval));
        assert!(!ApprovalState::Approved.can_transition_to(ApprovalState::Rejected));
        assert!(WaveState::Running.can_transition_to(WaveState::Blocked));
        assert!(!WaveState::Complete.can_transition_to(WaveState::Planned));
    }

    #[test]
    fn full_consensus_approval_requires_stage_shas_and_adr() {
        let approval = MigratePlanApprovalRecord {
            schema_version: MIGRATE_PLAN_APPROVAL_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            state: ApprovalState::PendingApproval,
            required_before_execution: true,
            target_spec_id: "spec-1".to_string(),
            target_spec_sha256: "sha".to_string(),
            approved_stage_artifacts: vec![],
            adr_sha256: None,
            requested_at: "2026-01-01T00:00:00Z".to_string(),
            approved_at: None,
            approved_by: None,
            approval_prompt_sha256: "prompt".to_string(),
        };
        assert!(approval.validate(PlanningMode::FullConsensus).is_err());
    }

    #[test]
    fn wave_validation_rejects_multi_app_and_cycles() {
        let index = MigratePlanWaveIndex {
            schema_version: MIGRATE_PLAN_WAVES_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            enabled: true,
            policy: "conditional_parallel".to_string(),
            multi_app_allowed: false,
            fallback_reason: Some("conflict".to_string()),
            waves: vec!["wave-a".to_string(), "wave-b".to_string()],
            dependency_graph_sha256: "sha".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let waves = vec![
            MigratePlanWaveRecord {
                schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
                run_id: "run-1".to_string(),
                app_key: "root-12345678".to_string(),
                wave_id: "wave-a".to_string(),
                wave_n: 1,
                stage_scope: "discover".to_string(),
                depends_on: vec!["wave-b".to_string()],
                dependency_graph: BTreeMap::from([
                    ("wave-a".to_string(), vec!["wave-b".to_string()]),
                    ("wave-b".to_string(), vec!["wave-a".to_string()]),
                ]),
                participants: vec!["root-12345678".to_string(), "other-87654321".to_string()],
                artifact_list: vec!["01-discover.md".to_string()],
                write_targets: vec!["run.json".to_string()],
                state: WaveState::Planned,
                receipts: vec![],
                independence_proof: vec!["disjoint inputs".to_string()],
                conflict_policy: "fallback_to_serial".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            },
            MigratePlanWaveRecord {
                schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
                run_id: "run-1".to_string(),
                app_key: "root-12345678".to_string(),
                wave_id: "wave-b".to_string(),
                wave_n: 2,
                stage_scope: "discover".to_string(),
                depends_on: vec!["wave-a".to_string()],
                dependency_graph: BTreeMap::from([
                    ("wave-a".to_string(), vec!["wave-b".to_string()]),
                    ("wave-b".to_string(), vec!["wave-a".to_string()]),
                ]),
                participants: vec!["root-12345678".to_string()],
                artifact_list: vec!["02-discover.md".to_string()],
                write_targets: vec!["approval.json".to_string()],
                state: WaveState::Planned,
                receipts: vec![],
                independence_proof: vec!["disjoint inputs".to_string()],
                conflict_policy: "fallback_to_serial".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            },
        ];
        assert!(validate_wave_plan(&index, &waves).is_err());
    }

    #[test]
    fn wave_validation_rejects_shared_write_targets() {
        let index = MigratePlanWaveIndex {
            schema_version: MIGRATE_PLAN_WAVES_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            enabled: true,
            policy: "conditional_parallel".to_string(),
            multi_app_allowed: false,
            fallback_reason: Some("conflict".to_string()),
            waves: vec!["wave-a".to_string(), "wave-b".to_string()],
            dependency_graph_sha256: "sha".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let waves = vec![
            MigratePlanWaveRecord {
                schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
                run_id: "run-1".to_string(),
                app_key: "root-12345678".to_string(),
                wave_id: "wave-a".to_string(),
                wave_n: 1,
                stage_scope: "reviewer".to_string(),
                depends_on: vec![],
                dependency_graph: BTreeMap::new(),
                participants: vec!["root-12345678".to_string()],
                artifact_list: vec!["05-reviewer.md".to_string()],
                write_targets: vec!["approval.json".to_string()],
                state: WaveState::Planned,
                receipts: vec![],
                independence_proof: vec!["disjoint read evidence".to_string()],
                conflict_policy: "fallback_to_serial".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            },
            MigratePlanWaveRecord {
                schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
                run_id: "run-1".to_string(),
                app_key: "root-12345678".to_string(),
                wave_id: "wave-b".to_string(),
                wave_n: 2,
                stage_scope: "reviewer".to_string(),
                depends_on: vec![],
                dependency_graph: BTreeMap::new(),
                participants: vec!["root-12345678".to_string()],
                artifact_list: vec!["06-reviewer.md".to_string()],
                write_targets: vec!["approval.json".to_string()],
                state: WaveState::Planned,
                receipts: vec![],
                independence_proof: vec!["disjoint read evidence".to_string()],
                conflict_policy: "fallback_to_serial".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            },
        ];
        assert!(validate_wave_plan(&index, &waves).is_err());
    }
}

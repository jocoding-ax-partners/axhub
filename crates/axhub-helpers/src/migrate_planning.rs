use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::atomic_jsonl;

pub const MIGRATE_PLANNING_PREVIEW_SCHEMA_VERSION: &str = "migrate-planning-preview/v1";
pub const MIGRATE_SPEC_LATEST_SCHEMA_VERSION: &str = "axhub/migrate-spec-latest/v1";
pub const MIGRATE_SPEC_META_SCHEMA_VERSION: &str = "axhub/migrate-spec/v1";
pub const MIGRATE_PLAN_RUN_SCHEMA_VERSION: &str = "axhub/migrate-plan-run/v1";
pub const MIGRATE_PLAN_APPROVAL_SCHEMA_VERSION: &str = "axhub/migrate-plan-approval/v1";
pub const MIGRATE_PLAN_STAGE_SCHEMA_VERSION: &str = "axhub/migrate-plan-stage/v1";
pub const MIGRATE_PLAN_APP_INDEX_SCHEMA_VERSION: &str = "axhub/migrate-plan-app-index/v1";
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
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Running => "running",
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::NeedsRevision => "needs_revision",
            Self::Rejected => "rejected",
            Self::Aborted => "aborted",
            Self::Superseded => "superseded",
        }
    }

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
    // #6 §5-required: emit `null` (not dropped) so strict consumers always see the key.
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
    // #6 §5-required: emit `[]` (not dropped) so strict consumers always see the key.
    #[serde(default)]
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
    // #6 §5-required: emit `null` (not dropped) so strict consumers always see the key.
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
    // #6 §5-required: emit `[]` (not dropped) so strict consumers always see the key.
    #[serde(default)]
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

#[derive(Debug, Serialize)]
pub struct StageWriteOutput {
    pub schema_version: String,
    pub stage: String,
    pub ordinal: u32,
    pub markdown_path: String,
    pub meta_path: Option<String>,
    pub run_state: String,
    pub approval_state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WavePlanOutput {
    pub schema_version: String,
    pub wave_id: String,
    pub wave_path: Option<String>,
    pub wave_index_path: Option<String>,
    pub parallelism_enabled: bool,
    pub serial_fallback: bool,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApproveOutput {
    pub schema_version: String,
    pub run_json_path: String,
    pub approval_json_path: String,
    pub spec_meta_path: String,
    pub latest_json_path: String,
    pub run_state: String,
    pub approval_state: String,
    pub spec_state: String,
}

pub fn run_migrate_stage_write(args: &[String]) -> Result<i32> {
    let mut run_json = None;
    let mut stage = None;
    let mut markdown_file = None;
    let mut summary = None;
    let mut run_state = None;
    let mut approval_state = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run-json" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --run-json 값이 필요해요");
                };
                run_json = Some(PathBuf::from(value));
                index += 2;
            }
            "--stage" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --stage 값이 필요해요");
                };
                stage = Some(value.clone());
                index += 2;
            }
            "--markdown-file" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --markdown-file 값이 필요해요");
                };
                markdown_file = Some(PathBuf::from(value));
                index += 2;
            }
            "--summary" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --summary 값이 필요해요");
                };
                summary = Some(value.clone());
                index += 2;
            }
            "--run-state" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --run-state 값이 필요해요");
                };
                run_state = Some(parse_run_state(value)?);
                index += 2;
            }
            "--approval-state" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-stage-write: --approval-state 값이 필요해요");
                };
                approval_state = Some(parse_approval_state(value)?);
                index += 2;
            }
            "--json" => {
                json = true;
                index += 1;
            }
            _ => bail!("migrate-stage-write: unknown option"),
        }
    }

    let output = migrate_stage_write(
        &required_path(run_json, "--run-json")?,
        &required_string(stage, "--stage")?,
        &required_path(markdown_file, "--markdown-file")?,
        summary.as_deref(),
        run_state,
        approval_state,
    )?;
    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("{}", output.markdown_path);
    }
    Ok(0)
}

pub fn run_migrate_wave_plan(args: &[String]) -> Result<i32> {
    let mut run_json = None;
    let mut wave_id = None;
    let mut stage_scope = None;
    let mut participants = Vec::new();
    let mut depends_on = Vec::new();
    let mut artifact_list = Vec::new();
    let mut write_targets = Vec::new();
    let mut independence_proof = Vec::new();
    let mut state = WaveState::Planned;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run-json" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --run-json 값이 필요해요");
                };
                run_json = Some(PathBuf::from(value));
                index += 2;
            }
            "--wave-id" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --wave-id 값이 필요해요");
                };
                wave_id = Some(value.clone());
                index += 2;
            }
            "--stage-scope" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --stage-scope 값이 필요해요");
                };
                stage_scope = Some(value.clone());
                index += 2;
            }
            "--participant" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --participant 값이 필요해요");
                };
                participants.push(value.clone());
                index += 2;
            }
            "--depends-on" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --depends-on 값이 필요해요");
                };
                depends_on.push(value.clone());
                index += 2;
            }
            "--artifact" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --artifact 값이 필요해요");
                };
                artifact_list.push(value.clone());
                index += 2;
            }
            "--write-target" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --write-target 값이 필요해요");
                };
                write_targets.push(value.clone());
                index += 2;
            }
            "--independence-proof" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --independence-proof 값이 필요해요");
                };
                independence_proof.push(value.clone());
                index += 2;
            }
            "--state" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-wave-plan: --state 값이 필요해요");
                };
                state = parse_wave_state(value)?;
                index += 2;
            }
            "--json" => {
                json = true;
                index += 1;
            }
            _ => bail!("migrate-wave-plan: unknown option"),
        }
    }

    let output = migrate_wave_plan(
        &required_path(run_json, "--run-json")?,
        &required_string(wave_id, "--wave-id")?,
        &required_string(stage_scope, "--stage-scope")?,
        participants,
        depends_on,
        artifact_list,
        write_targets,
        independence_proof,
        state,
    )?;
    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else if let Some(path) = output.wave_path.as_deref() {
        println!("{}", path);
    } else {
        println!("serial-fallback");
    }
    Ok(0)
}

pub fn run_migrate_approve(args: &[String]) -> Result<i32> {
    let mut run_json = None;
    let mut approved_by = None;
    let mut approval_note = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run-json" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-approve: --run-json 값이 필요해요");
                };
                run_json = Some(PathBuf::from(value));
                index += 2;
            }
            "--approved-by" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-approve: --approved-by 값이 필요해요");
                };
                approved_by = Some(value.clone());
                index += 2;
            }
            "--approval-note" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-approve: --approval-note 값이 필요해요");
                };
                approval_note = Some(value.clone());
                index += 2;
            }
            "--json" => {
                json = true;
                index += 1;
            }
            _ => bail!("migrate-approve: unknown option"),
        }
    }

    let output = migrate_approve(
        &required_path(run_json, "--run-json")?,
        &required_string(approved_by, "--approved-by")?,
        approval_note.as_deref(),
    )?;
    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("{}", output.latest_json_path);
    }
    Ok(0)
}

pub fn migrate_stage_write(
    run_json_path: &Path,
    stage: &str,
    markdown_file: &Path,
    summary: Option<&str>,
    run_state: Option<RunState>,
    approval_state: Option<ApprovalState>,
) -> Result<StageWriteOutput> {
    let mut run = read_json::<MigratePlanRunRecord>(run_json_path)?;
    run.validate()?;
    run.assert_repo_fingerprint_matches(Path::new(&run.repo_root))?;
    if run.mode != PlanningMode::FullConsensus {
        bail!("migrate-stage-write: full_consensus run 만 stage artifact 를 써요");
    }
    let run_dir = run_json_path
        .parent()
        .context("migrate-stage-write: run_json parent 가 없어요")?;
    let approval_path = run_dir.join("approval.json");
    let mut approval = read_json::<MigratePlanApprovalRecord>(&approval_path)?;
    let now = now_ts();

    // Refuse agent-authored drafts placed inside stages/: the helper owns that
    // directory. Feeding such a file back as --markdown-file would clone it under
    // the next ordinal (the duplicate-stage-md regression).
    let stages_dir_guard = run_dir.join("stages");
    let canonical_md = markdown_file
        .canonicalize()
        .unwrap_or_else(|_| markdown_file.to_path_buf());
    let canonical_stages = stages_dir_guard
        .canonicalize()
        .unwrap_or_else(|_| stages_dir_guard.clone());
    if canonical_md.starts_with(&canonical_stages) {
        bail!(
            "migrate-stage-write: --markdown-file 은 stages/ 밖(예: <run_dir>/drafts/)에 두세요 — stages/ 는 helper 전용 경로예요"
        );
    }

    let raw_markdown = fs::read_to_string(markdown_file).with_context(|| {
        format!(
            "{} stage markdown 를 읽지 못했어요",
            markdown_file.display()
        )
    })?;
    // Backstop: never persist secrets to disk.
    // redacted = secret mask 가 실제로 발화했는지 (정규화-only 변경은 제외).
    let (markdown, was_redacted) = crate::redact::redact_with_mask_flag(&raw_markdown);
    // SHA is computed over the redacted bytes — the same content written to disk —
    // so the approval seal hash matches what `collect_stage_sha_list` later reads.
    let markdown_sha = sha256_hex(&markdown);

    let (markdown_target, meta_target, event_name) = if stage == "adr" {
        (run_dir.join("adr.md"), None, "adr_written")
    } else {
        let ordinal = stage_fixed_ordinal(stage)?;
        let stages_dir = run_dir.join("stages");
        fs::create_dir_all(&stages_dir)?;
        (
            stages_dir.join(format!("{ordinal:02}-{stage}.md")),
            Some(stages_dir.join(format!("{ordinal:02}-{stage}.meta.json"))),
            "stage_written",
        )
    };

    // md 와 meta 는 각각 atomic write 라 중간 실패 시 meta 가 stale 할 수 있어요.
    // seal(collect_stage_sha_list)은 디스크 md 를 재해싱하므로 진실원천은 md 본문이고,
    // meta 의 revision/redacted 는 best-effort 감사 메타데이터예요.
    write_text_atomically(&markdown_target, &markdown)?;
    if let Some(meta_path) = meta_target.as_ref() {
        let ordinal = stage_ordinal_from_path(&markdown_target)?;
        // Idempotent overwrite: re-recording the same stage bumps `revision`
        // instead of producing a new file, so drift is visible without duplication.
        let revision = if meta_path.exists() {
            read_json::<serde_json::Value>(meta_path)
                .ok()
                .and_then(|v| v.get("revision").and_then(serde_json::Value::as_u64))
                .unwrap_or(0)
                + 1
        } else {
            1
        };
        let meta = json!({
            "schema_version": MIGRATE_PLAN_STAGE_SCHEMA_VERSION,
            "run_id": run.run_id,
            "app_key": run.app_key,
            "stage": stage,
            "stage_n": ordinal,
            "revision": revision,
            "redacted": was_redacted,
            "state": "complete",
            "artifact_sha256": markdown_sha,
            "created_at": now,
            "updated_at": now
        });
        write_json_atomically(meta_path, &meta)?;
    }

    let receipt_summary = summary
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{stage} artifact written"));
    append_receipt(
        &run_dir.join("receipts.jsonl"),
        &now,
        event_name,
        if stage == "adr" { None } else { Some(stage) },
        if stage == "adr" {
            None
        } else {
            Some(stage_ordinal_from_path(&markdown_target)?)
        },
        &markdown_target,
        &markdown_sha,
        &receipt_summary,
    )?;

    let effective_run_state = if let Some(next) = run_state {
        if run.state != next && !run.state.can_transition_to(next) {
            bail!("migrate-stage-write: invalid run state transition");
        }
        run.state = next;
        next
    } else {
        run.state
    };
    run.updated_at = now.clone();

    let effective_approval_state = if let Some(next) = approval_state {
        if approval.state != next && !approval.state.can_transition_to(next) {
            bail!("migrate-stage-write: invalid approval state transition");
        }
        approval.state = next;
        if next == ApprovalState::PendingApproval && run.mode == PlanningMode::FullConsensus {
            // §8.10: refuse to seal running → pending_approval while any wave is
            // in-progress-unfinished. `planned`/`complete` waves are acceptable at seal.
            let seal_waves = load_existing_waves(&run_dir.join("waves"))?;
            if let Some(blocking) = seal_waves.iter().find(|wave| {
                matches!(
                    wave.state,
                    WaveState::Running | WaveState::NeedsRevision | WaveState::Blocked
                )
            }) {
                bail!(
                    "migrate-stage-write: 진행 중 wave 가 있어 seal 할 수 없어요 ({} = {:?}) (§8.10)",
                    blocking.wave_id,
                    blocking.state
                );
            }
            approval.approved_stage_artifacts = collect_stage_sha_list(&run_dir.join("stages"))?;
            let adr_path = run_dir.join("adr.md");
            if !adr_path.exists() {
                bail!("migrate-stage-write: full_consensus approval 전에는 adr.md 가 필요해요");
            }
            approval.adr_sha256 = Some(sha256_hex(&fs::read_to_string(&adr_path)?));
        }
        next
    } else {
        approval.state
    };

    let (spec_markdown_path, spec_meta_path, _) =
        spec_artifact_paths(run_dir, &run.app_key, &approval.target_spec_id)?;
    write_json_atomically(run_json_path, &serde_json::to_value(&run)?)?;
    approval.requested_at = now.clone();
    write_json_atomically(&approval_path, &serde_json::to_value(&approval)?)?;
    if effective_approval_state == ApprovalState::PendingApproval {
        update_spec_meta_state(
            &spec_meta_path,
            "pending_approval",
            Some(effective_approval_state.state_str()),
            None,
            None,
            None,
            &now,
        )?;
    } else if !spec_markdown_path.exists() {
        bail!("migrate-stage-write: target spec markdown 를 찾지 못했어요");
    }
    // #3 restructure: the spec stays `draft` through the revision loop. The run/approval
    // states carry the revision signal (via the wired Run guard above), so the over-propagating
    // `draft → needs_revision` spec-meta write was removed. The only production spec transition
    // on this path is now `draft → pending_approval` (legal per the §7 SpecState matrix).
    update_latest_run_state(&run, run_dir)?;

    if approval.state == ApprovalState::PendingApproval {
        append_receipt(
            &run_dir.join("receipts.jsonl"),
            &now,
            "approval_requested",
            None,
            None,
            &approval_path,
            &sha256_hex(&serde_json::to_string(&approval)?),
            "consensus review complete; pending approval",
        )?;
    }

    Ok(StageWriteOutput {
        schema_version: "migrate-stage-write/v1".to_string(),
        stage: stage.to_string(),
        ordinal: if stage == "adr" {
            0
        } else {
            stage_ordinal_from_path(&markdown_target)?
        },
        markdown_path: markdown_target.display().to_string(),
        meta_path: meta_target.map(|path| path.display().to_string()),
        run_state: effective_run_state.as_str().to_string(),
        approval_state: Some(effective_approval_state.state_str().to_string()),
    })
}

pub fn migrate_approve(
    run_json_path: &Path,
    approved_by: &str,
    approval_note: Option<&str>,
) -> Result<ApproveOutput> {
    let mut run = read_json::<MigratePlanRunRecord>(run_json_path)?;
    run.validate()?;
    run.assert_repo_fingerprint_matches(Path::new(&run.repo_root))?;
    if run.mode == PlanningMode::Simple {
        bail!("migrate-approve: simple flow 는 planning approval 대상이 아니에요");
    }
    if run.state != RunState::PendingApproval {
        bail!("migrate-approve: pending_approval run 에서만 승인할 수 있어요");
    }
    let run_dir = run_json_path
        .parent()
        .context("migrate-approve: run_json parent 가 없어요")?;
    let approval_path = run_dir.join("approval.json");
    let mut approval = read_json::<MigratePlanApprovalRecord>(&approval_path)?;
    approval.validate(run.mode)?;
    if approval.state != ApprovalState::PendingApproval {
        bail!("migrate-approve: approval state 가 pending_approval 이어야 해요");
    }

    let now = now_ts();
    let approval_hash = sha256_hex(approval_note.unwrap_or("approved"));
    let (spec_markdown_path, spec_meta_path, latest_json_path) =
        spec_artifact_paths(run_dir, &run.app_key, &approval.target_spec_id)?;
    let spec_markdown = fs::read_to_string(&spec_markdown_path).with_context(|| {
        format!(
            "{} spec markdown 를 읽지 못했어요",
            spec_markdown_path.display()
        )
    })?;
    let spec_sha = sha256_hex(&spec_markdown);
    if spec_sha != approval.target_spec_sha256 {
        bail!("migrate-approve: target spec sha 가 approval record 와 달라요");
    }

    if run.state != RunState::Approved && !run.state.can_transition_to(RunState::Approved) {
        bail!("migrate-approve: invalid run state transition");
    }
    run.state = RunState::Approved;
    run.updated_at = now.clone();

    if approval.state != ApprovalState::Approved
        && !approval.state.can_transition_to(ApprovalState::Approved)
    {
        bail!("migrate-approve: invalid approval state transition");
    }
    approval.state = ApprovalState::Approved;
    approval.approved_at = Some(now.clone());
    approval.approved_by = Some(approved_by.to_string());
    approval.approval_prompt_sha256 = approval_hash.clone();

    write_json_atomically(run_json_path, &serde_json::to_value(&run)?)?;
    write_json_atomically(&approval_path, &serde_json::to_value(&approval)?)?;
    update_spec_meta_state(
        &spec_meta_path,
        "approved",
        Some("approved"),
        Some(&now),
        Some(approved_by),
        Some(&approval_hash),
        &now,
    )?;

    let latest = json!({
        "schema_version": MIGRATE_SPEC_LATEST_SCHEMA_VERSION,
        "app_key": run.app_key,
        "latest_spec_id": approval.target_spec_id,
        "latest_spec_path": spec_markdown_path.display().to_string(),
        "source_plan_run_id": run.run_id,
        "approval_state": "approved",
        "approved_at": now,
        "approved_by": approved_by,
        "approval_prompt_sha256": approval_hash,
        "sha256": spec_sha,
        "updated_at": now
    });
    write_json_atomically(&latest_json_path, &latest)?;
    update_latest_run_state(&run, run_dir)?;

    append_receipt(
        &run_dir.join("receipts.jsonl"),
        &now,
        "approval_approved",
        None,
        None,
        &approval_path,
        &sha256_hex(&serde_json::to_string(&approval)?),
        "migrate planning approved",
    )?;
    append_receipt(
        &run_dir.join("receipts.jsonl"),
        &now,
        "spec_latest_promoted",
        None,
        None,
        &latest_json_path,
        &sha256_hex(&serde_json::to_string(&latest)?),
        "approved spec promoted to latest pointer",
    )?;

    Ok(ApproveOutput {
        schema_version: "migrate-approve/v1".to_string(),
        run_json_path: run_json_path.display().to_string(),
        approval_json_path: approval_path.display().to_string(),
        spec_meta_path: spec_meta_path.display().to_string(),
        latest_json_path: latest_json_path.display().to_string(),
        run_state: run.state.as_str().to_string(),
        approval_state: approval.state.state_str().to_string(),
        spec_state: "approved".to_string(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn migrate_wave_plan(
    run_json_path: &Path,
    wave_id: &str,
    stage_scope: &str,
    participants: Vec<String>,
    depends_on: Vec<String>,
    artifact_list: Vec<String>,
    write_targets: Vec<String>,
    independence_proof: Vec<String>,
    state: WaveState,
) -> Result<WavePlanOutput> {
    let mut run = read_json::<MigratePlanRunRecord>(run_json_path)?;
    run.validate()?;
    run.assert_repo_fingerprint_matches(Path::new(&run.repo_root))?;
    if run.mode != PlanningMode::FullConsensus {
        bail!("migrate-wave-plan: full_consensus run 에서만 wave 를 써요");
    }
    let run_dir = run_json_path
        .parent()
        .context("migrate-wave-plan: run_json parent 가 없어요")?;
    let waves_dir = run_dir.join("waves");
    let wave_path = waves_dir.join(format!("{wave_id}.json"));
    let index_path = waves_dir.join("waves.json");
    let now = now_ts();
    let participant_list = if participants.is_empty() {
        vec![run.app_key.clone()]
    } else {
        participants
    };

    let mut existing = load_existing_waves(&waves_dir)?;
    // #3 §7 WaveState guard (with idempotency escape). Capture the prior state BEFORE `retain`
    // discards it: `None` is a fresh wave (must be born `planned`); `Some(prior)` allows the
    // idempotent same-state upsert (`prior == state`) plus any legal §7 transition.
    let prior_wave_state = existing
        .iter()
        .find(|wave| wave.wave_id == wave_id)
        .map(|wave| wave.state);
    match prior_wave_state {
        None => {
            if state != WaveState::Planned {
                bail!(
                    "migrate-wave-plan: 새 wave 는 planned 상태로 시작해야 해요 (none → {state:?})"
                );
            }
        }
        Some(prior) => {
            if prior != state && !prior.can_transition_to(state) {
                bail!("migrate-wave-plan: invalid wave state transition ({prior:?} → {state:?})");
            }
        }
    }
    // #2 §8.9 (candidate-scoped, before `existing.push`): a `complete` wave must declare
    // artifact backing that actually exists on disk. The record has no sha/output_ref field
    // and `receipts` is always empty, so `artifact_list` + fs-existence is the only signal.
    if state == WaveState::Complete {
        if artifact_list.is_empty() {
            bail!("migrate-wave-plan: complete wave 는 artifact_list backing 이 필요해요 (§8.9)");
        }
        for artifact in &artifact_list {
            if !run_dir.join(artifact).exists() {
                bail!("migrate-wave-plan: complete wave artifact 가 디스크에 없어요: {artifact} (§8.9)");
            }
        }
    }
    existing.retain(|wave| wave.wave_id != wave_id);
    let graph_seed = existing
        .iter()
        .map(|wave| (wave.wave_id.clone(), wave.depends_on.clone()))
        .collect::<BTreeMap<_, _>>();
    let candidate_wave = MigratePlanWaveRecord {
        schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
        run_id: run.run_id.clone(),
        app_key: run.app_key.clone(),
        wave_id: wave_id.to_string(),
        wave_n: (existing.len() + 1) as u32,
        stage_scope: stage_scope.to_string(),
        depends_on,
        dependency_graph: graph_seed,
        participants: participant_list,
        artifact_list,
        write_targets,
        state,
        receipts: vec![],
        independence_proof,
        conflict_policy: "fallback_to_serial".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    existing.push(candidate_wave);
    let graph = existing
        .iter()
        .map(|wave| (wave.wave_id.clone(), wave.depends_on.clone()))
        .collect::<BTreeMap<_, _>>();
    let waves = existing
        .into_iter()
        .map(|mut wave| {
            wave.dependency_graph = graph.clone();
            wave
        })
        .collect::<Vec<_>>();
    let index = MigratePlanWaveIndex {
        schema_version: MIGRATE_PLAN_WAVES_SCHEMA_VERSION.to_string(),
        run_id: run.run_id.clone(),
        app_key: run.app_key.clone(),
        enabled: true,
        policy: "conditional_parallel".to_string(),
        multi_app_allowed: false,
        fallback_reason: None,
        waves: waves.iter().map(|wave| wave.wave_id.clone()).collect(),
        dependency_graph_sha256: sha256_hex(&serde_json::to_string(&graph)?),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    match validate_wave_plan(&index, &waves) {
        Ok(()) => {
            fs::create_dir_all(&waves_dir)?;
            for wave in &waves {
                write_json_atomically(
                    &waves_dir.join(format!("{}.json", wave.wave_id)),
                    &serde_json::to_value(wave)?,
                )?;
            }
            write_json_atomically(&index_path, &serde_json::to_value(&index)?)?;
            run.parallelism.enabled = true;
            run.parallelism.fallback_reason = None;
            run.wave_index_path = Some(index_path.display().to_string());
            run.updated_at = now.clone();
            write_json_atomically(run_json_path, &serde_json::to_value(&run)?)?;
            update_latest_run_state(&run, run_dir)?;
            append_receipt(
                &run_dir.join("receipts.jsonl"),
                &now,
                "wave_planned",
                Some(stage_scope),
                Some(waves.len() as u32),
                &wave_path,
                &sha256_hex(&serde_json::to_string(&waves)?),
                "same-app wave planned",
            )?;
            Ok(WavePlanOutput {
                schema_version: "migrate-wave-plan/v1".to_string(),
                wave_id: wave_id.to_string(),
                wave_path: Some(wave_path.display().to_string()),
                wave_index_path: Some(index_path.display().to_string()),
                parallelism_enabled: true,
                serial_fallback: false,
                fallback_reason: None,
            })
        }
        Err(err) => {
            run.parallelism.enabled = false;
            run.parallelism.fallback_reason = Some(err.to_string());
            run.wave_index_path = None;
            run.updated_at = now.clone();
            write_json_atomically(run_json_path, &serde_json::to_value(&run)?)?;
            update_latest_run_state(&run, run_dir)?;
            append_receipt(
                &run_dir.join("receipts.jsonl"),
                &now,
                "wave_serial_fallback",
                Some(stage_scope),
                None,
                run_json_path,
                &sha256_hex(&serde_json::to_string(&run)?),
                &format!("serial fallback: {err}"),
            )?;
            Ok(WavePlanOutput {
                schema_version: "migrate-wave-plan/v1".to_string(),
                wave_id: wave_id.to_string(),
                wave_path: None,
                wave_index_path: None,
                parallelism_enabled: false,
                serial_fallback: true,
                fallback_reason: Some(err.to_string()),
            })
        }
    }
}

impl ApprovalState {
    fn state_str(self) -> &'static str {
        match self {
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::NeedsRevision => "needs_revision",
        }
    }
}

fn parse_run_state(value: &str) -> Result<RunState> {
    match value {
        "draft" => Ok(RunState::Draft),
        "running" => Ok(RunState::Running),
        "pending_approval" => Ok(RunState::PendingApproval),
        "approved" => Ok(RunState::Approved),
        "needs_revision" => Ok(RunState::NeedsRevision),
        "rejected" => Ok(RunState::Rejected),
        "aborted" => Ok(RunState::Aborted),
        "superseded" => Ok(RunState::Superseded),
        _ => bail!("migrate planning: unknown run state"),
    }
}

fn parse_approval_state(value: &str) -> Result<ApprovalState> {
    match value {
        "pending_approval" => Ok(ApprovalState::PendingApproval),
        "approved" => Ok(ApprovalState::Approved),
        "rejected" => Ok(ApprovalState::Rejected),
        "needs_revision" => Ok(ApprovalState::NeedsRevision),
        _ => bail!("migrate planning: unknown approval state"),
    }
}

fn parse_wave_state(value: &str) -> Result<WaveState> {
    match value {
        "planned" => Ok(WaveState::Planned),
        "running" => Ok(WaveState::Running),
        "complete" => Ok(WaveState::Complete),
        "needs_revision" => Ok(WaveState::NeedsRevision),
        "blocked" => Ok(WaveState::Blocked),
        "aborted" => Ok(WaveState::Aborted),
        _ => bail!("migrate planning: unknown wave state"),
    }
}

fn parse_spec_state(value: &str) -> Result<SpecState> {
    match value {
        "draft" => Ok(SpecState::Draft),
        "pending_approval" => Ok(SpecState::PendingApproval),
        "approved" => Ok(SpecState::Approved),
        "needs_revision" => Ok(SpecState::NeedsRevision),
        "rejected" => Ok(SpecState::Rejected),
        "superseded" => Ok(SpecState::Superseded),
        _ => bail!("migrate planning: unknown spec state"),
    }
}

fn required_path(value: Option<PathBuf>, flag: &str) -> Result<PathBuf> {
    value.with_context(|| format!("migrate planning: {flag} 값이 필요해요"))
}

fn required_string(value: Option<String>, flag: &str) -> Result<String> {
    let value = value.with_context(|| format!("migrate planning: {flag} 값이 필요해요"))?;
    if value.trim().is_empty() {
        bail!("migrate planning: {flag} 값이 비어 있어요");
    }
    Ok(value)
}

fn stage_fixed_ordinal(stage: &str) -> Result<u32> {
    match stage {
        "discover" => Ok(1),
        "planner" => Ok(2),
        "architect" => Ok(3),
        "critic" => Ok(4),
        "reviewer" => Ok(5),
        _ => bail!("migrate-stage-write: 지원하지 않는 stage 예요"),
    }
}

fn stage_ordinal_from_path(path: &Path) -> Result<u32> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("migrate planning: stage file name 이 없어요")?;
    let prefix = name
        .split_once('-')
        .map(|(prefix, _)| prefix)
        .context("migrate planning: stage file prefix 를 읽지 못했어요")?;
    prefix
        .parse::<u32>()
        .with_context(|| format!("migrate planning: invalid stage ordinal {prefix}"))
}

fn collect_stage_sha_list(stages_dir: &Path) -> Result<Vec<String>> {
    if !stages_dir.exists() {
        return Ok(vec![]);
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(stages_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        items.push(format!("{}:{}", path.display(), sha256_hex(&raw)));
    }
    items.sort();
    Ok(items)
}

fn load_existing_waves(waves_dir: &Path) -> Result<Vec<MigratePlanWaveRecord>> {
    if !waves_dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(waves_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("waves.json") {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        out.push(read_json::<MigratePlanWaveRecord>(&path)?);
    }
    out.sort_by(|left, right| left.wave_id.cmp(&right.wave_id));
    Ok(out)
}

fn update_latest_run_state(run: &MigratePlanRunRecord, run_dir: &Path) -> Result<()> {
    let plan_root = run_dir
        .parent()
        .and_then(|path| path.parent())
        .context("migrate planning: latest-run root 를 계산하지 못했어요")?;
    let latest_run = plan_root
        .join("apps")
        .join(&run.app_key)
        .join("latest-run.json");
    let payload = json!({
        "schema_version": MIGRATE_PLAN_APP_INDEX_SCHEMA_VERSION,
        "app_key": run.app_key,
        "latest_run_id": run.run_id,
        "latest_run_path": run_dir.display().to_string(),
        "run_state": run.state.as_str(),
        "repo_fingerprint": run.repo_fingerprint,
        "updated_at": run.updated_at
    });
    write_json_atomically(&latest_run, &payload)
}

fn spec_artifact_paths(
    run_dir: &Path,
    app_key: &str,
    spec_id: &str,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let planning_root = run_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .context("migrate planning: spec root 를 계산하지 못했어요")?;
    let spec_app_dir = planning_root.join("spec").join("apps").join(app_key);
    let spec_dir = spec_app_dir.join("specs");
    Ok((
        spec_dir.join(format!("{spec_id}.md")),
        spec_dir.join(format!("{spec_id}.meta.json")),
        spec_app_dir.join("latest.json"),
    ))
}

fn update_spec_meta_state(
    spec_meta_path: &Path,
    status: &str,
    approval_state: Option<&str>,
    approved_at: Option<&str>,
    approved_by: Option<&str>,
    approval_prompt_sha256: Option<&str>,
    updated_at: &str,
) -> Result<()> {
    let mut value = read_json::<Value>(spec_meta_path)?;
    // #3 §7 SpecState guard (with idempotency escape). The matrices have no reflexive edges,
    // so the `current != next` escape is required for legal same-state re-writes (e.g.
    // migrate_approve's re-approve `approved → approved`). Skip silently if either status is
    // unparseable so legacy/foreign meta files are not hard-failed.
    if let Some(current_str) = value["status"].as_str() {
        if let (Ok(current), Ok(next)) = (parse_spec_state(current_str), parse_spec_state(status)) {
            if current != next && !current.can_transition_to(next) {
                bail!("migrate planning: invalid spec state transition ({current_str} → {status})");
            }
        }
    }
    value["status"] = Value::String(status.to_string());
    value["updated_at"] = Value::String(updated_at.to_string());
    if let Some(state) = approval_state {
        value["approval"]["state"] = Value::String(state.to_string());
    }
    value["approval"]["approved_at"] = approved_at
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    value["approval"]["approved_by"] = approved_by
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    if let Some(prompt_sha) = approval_prompt_sha256 {
        value["approval"]["approval_prompt_sha256"] = Value::String(prompt_sha.to_string());
    }
    write_json_atomically(spec_meta_path, &value)
}

fn write_json_atomically(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn write_text_atomically(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("{} 파일을 읽지 못했어요", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("{} JSON 을 해석하지 못했어요", path.display()))
}

#[allow(clippy::too_many_arguments)]
fn append_receipt(
    receipts_path: &Path,
    now: &str,
    event: &str,
    stage: Option<&str>,
    stage_n: Option<u32>,
    artifact_path: &Path,
    sha256: &str,
    summary: &str,
) -> Result<()> {
    let line = json!({
        "ts": now,
        "event": event,
        "stage": stage,
        "stage_n": stage_n,
        "artifact_path": artifact_path.display().to_string(),
        "sha256": sha256,
        "summary": summary,
        "redacted": true
    });
    atomic_jsonl::append_line(receipts_path, &serde_json::to_string(&line)?)?;
    Ok(())
}

fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
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

    // A valid spec_only run that passes `validate()`. Each #7/§8 reach-test mutates exactly
    // one field so the injected defect is the only reason the validator can bail.
    fn spec_only_run_fixture() -> MigratePlanRunRecord {
        MigratePlanRunRecord {
            schema_version: MIGRATE_PLAN_RUN_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            app_path: ".".to_string(),
            repo_root: "/repo".to_string(),
            remote_repo: None,
            r#ref: None,
            owned_app_keys: Some(vec!["root-12345678".to_string()]),
            mode: PlanningMode::SpecOnly,
            stage_order: vec![],
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
            repo_fingerprint: "abc".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn spec_only_run_fixture_is_valid() {
        // Guards the reach-tests below: the baseline must pass so each mutation is the sole defect.
        spec_only_run_fixture().validate().unwrap();
    }

    #[test]
    fn run_record_rejects_multi_owned_app_keys() {
        // #7 split: the owned_app_keys defect is reached on its own (no `||` short-circuit).
        let mut record = spec_only_run_fixture();
        record.owned_app_keys = Some(vec![
            "root-12345678".to_string(),
            "other-87654321".to_string(),
        ]);
        let error = record.validate().unwrap_err().to_string();
        assert!(
            error.contains("owned_app_keys"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn run_record_rejects_spec_only_parallelism() {
        // #7 split: the spec_only parallelism branch is reached independently.
        let mut record = spec_only_run_fixture();
        record.parallelism.enabled = true;
        let error = record.validate().unwrap_err().to_string();
        assert!(error.contains("serial"), "unexpected error: {error}");
    }

    #[test]
    fn run_record_rejects_spec_only_wave_metadata() {
        // §8.4: spec_only must not declare wave metadata (reaches the wave_index_path validator).
        let mut record = spec_only_run_fixture();
        record.wave_index_path = Some("/repo/.axhub/plan/runs/run-1/waves/waves.json".to_string());
        let error = record.validate().unwrap_err().to_string();
        assert!(error.contains("wave metadata"), "unexpected error: {error}");
    }

    #[test]
    fn run_record_rejects_multi_app_allowed() {
        // §8.2: multi_app_allowed must remain false in v1.
        let mut record = spec_only_run_fixture();
        record.wave_policy.multi_app_allowed = true;
        let error = record.validate().unwrap_err().to_string();
        assert!(error.contains("multi_app"), "unexpected error: {error}");
    }

    #[test]
    fn required_fields_serialize_as_empty_not_dropped() {
        // #6: §5-required fields emit []/null even when empty, instead of being dropped by
        // `skip_serializing_if`, so a strict consumer always sees the key.
        let run_json = serde_json::to_value(spec_only_run_fixture()).unwrap();
        assert!(
            run_json.get("hard_stop_reasons").is_some(),
            "run.json hard_stop_reasons key was dropped"
        );
        assert!(run_json["hard_stop_reasons"].is_array());
        assert!(
            run_json["parallelism"].get("fallback_reason").is_some(),
            "run.json parallelism.fallback_reason key was dropped"
        );
        assert!(run_json["parallelism"]["fallback_reason"].is_null());

        let index = MigratePlanWaveIndex {
            schema_version: MIGRATE_PLAN_WAVES_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            enabled: true,
            policy: "conditional_parallel".to_string(),
            multi_app_allowed: false,
            fallback_reason: None,
            waves: vec![],
            dependency_graph_sha256: "sha".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let index_json = serde_json::to_value(&index).unwrap();
        assert!(
            index_json.get("fallback_reason").is_some(),
            "waves.json fallback_reason key was dropped"
        );
        assert!(index_json["fallback_reason"].is_null());

        let wave = MigratePlanWaveRecord {
            schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            wave_id: "wave-a".to_string(),
            wave_n: 1,
            stage_scope: "reviewer".to_string(),
            depends_on: vec![],
            dependency_graph: BTreeMap::new(),
            participants: vec!["root-12345678".to_string()],
            artifact_list: vec![],
            write_targets: vec![],
            state: WaveState::Planned,
            receipts: vec![],
            independence_proof: vec!["disjoint inputs".to_string()],
            conflict_policy: "fallback_to_serial".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let wave_json = serde_json::to_value(&wave).unwrap();
        assert!(
            wave_json.get("receipts").is_some(),
            "wave record receipts key was dropped"
        );
        assert!(wave_json["receipts"].is_array());
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
    fn wave_validation_rejects_multi_app_participant() {
        // #7 split / §8.3: a participant outside the run app_key is the sole, identifiable defect.
        let index = MigratePlanWaveIndex {
            schema_version: MIGRATE_PLAN_WAVES_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            enabled: true,
            policy: "conditional_parallel".to_string(),
            multi_app_allowed: false,
            fallback_reason: Some("conflict".to_string()),
            waves: vec!["wave-a".to_string()],
            dependency_graph_sha256: "sha".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let waves = vec![MigratePlanWaveRecord {
            schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: "root-12345678".to_string(),
            wave_id: "wave-a".to_string(),
            wave_n: 1,
            stage_scope: "discover".to_string(),
            depends_on: vec![],
            dependency_graph: BTreeMap::new(),
            participants: vec!["root-12345678".to_string(), "other-87654321".to_string()],
            artifact_list: vec!["01-discover.md".to_string()],
            write_targets: vec!["run.json".to_string()],
            state: WaveState::Planned,
            receipts: vec![],
            independence_proof: vec!["disjoint inputs".to_string()],
            conflict_policy: "fallback_to_serial".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }];
        let error = validate_wave_plan(&index, &waves).unwrap_err().to_string();
        assert!(
            error.contains("participants must share the run app_key"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn wave_validation_rejects_dependency_cycles() {
        // #7 split / §8.7: a depends_on cycle is the sole, identifiable defect (participants valid).
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
                dependency_graph: BTreeMap::new(),
                participants: vec!["root-12345678".to_string()],
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
                dependency_graph: BTreeMap::new(),
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
        let error = validate_wave_plan(&index, &waves).unwrap_err().to_string();
        assert!(
            error.contains("dependency cycles are not allowed"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn wave_validation_rejects_same_wave_target_overlap_per_category() {
        // #2 §8.5: a target appearing twice in ONE wave is rejected, across all 7 target classes.
        let categories = [
            ("artifact_path", "stages/05-reviewer.md"),
            (
                "latest_pointer",
                ".axhub/spec/apps/root-12345678/latest.json",
            ),
            (
                "plan_app_index",
                ".axhub/plan/apps/root-12345678/latest-run.json",
            ),
            (
                "spec_app_dir",
                ".axhub/spec/apps/root-12345678/specs/spec-1.md",
            ),
            ("run_metadata", "run.json"),
            ("approval_metadata", "approval.json"),
            ("receipt_target", "receipts.jsonl"),
        ];
        for (category, target) in categories {
            let index = MigratePlanWaveIndex {
                schema_version: MIGRATE_PLAN_WAVES_SCHEMA_VERSION.to_string(),
                run_id: "run-1".to_string(),
                app_key: "root-12345678".to_string(),
                enabled: true,
                policy: "conditional_parallel".to_string(),
                multi_app_allowed: false,
                fallback_reason: None,
                waves: vec!["wave-a".to_string()],
                dependency_graph_sha256: "sha".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            };
            let waves = vec![MigratePlanWaveRecord {
                schema_version: MIGRATE_PLAN_WAVE_SCHEMA_VERSION.to_string(),
                run_id: "run-1".to_string(),
                app_key: "root-12345678".to_string(),
                wave_id: "wave-a".to_string(),
                wave_n: 1,
                stage_scope: "reviewer".to_string(),
                depends_on: vec![],
                dependency_graph: BTreeMap::new(),
                participants: vec!["root-12345678".to_string()],
                artifact_list: vec![],
                // same target twice inside one wave -> §8.5 overlap.
                write_targets: vec![target.to_string(), target.to_string()],
                state: WaveState::Planned,
                receipts: vec![],
                independence_proof: vec!["disjoint inputs".to_string()],
                conflict_policy: "fallback_to_serial".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            }];
            let error = validate_wave_plan(&index, &waves).unwrap_err().to_string();
            assert!(
                error.contains("duplicate write target inside one wave"),
                "{category}: unexpected error: {error}"
            );
        }
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

    #[test]
    fn update_spec_meta_state_rejects_illegal_transition() {
        let temp = tempfile::tempdir().unwrap();
        let meta = temp.path().join("spec.meta.json");
        write_json_atomically(
            &meta,
            &json!({"status": "approved", "approval": {"state": "approved"}}),
        )
        .unwrap();
        // approved -> needs_revision is illegal per the SpecState matrix (Approved -> Superseded only).
        let error = update_spec_meta_state(
            &meta,
            "needs_revision",
            Some("needs_revision"),
            None,
            None,
            None,
            "2026-01-01T00:00:00Z",
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("invalid spec state transition"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn update_spec_meta_state_allows_idempotent_same_state() {
        let temp = tempfile::tempdir().unwrap();
        let meta = temp.path().join("spec.meta.json");
        write_json_atomically(
            &meta,
            &json!({"status": "approved", "approval": {"state": "approved"}}),
        )
        .unwrap();
        // approved -> approved must NOT bail: the matrices have no reflexive edges, so the
        // `current != next` escape (mirroring migrate_approve's re-approve path) is required.
        update_spec_meta_state(
            &meta,
            "approved",
            Some("approved"),
            None,
            None,
            None,
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
    }

    /// Scaffold a full_consensus run on disk and return the run.json path.
    ///
    /// Layout mirrors production: `<planning_root>/plan/runs/<run_id>/run.json`
    /// (run_dir 3 levels under planning_root) so `spec_artifact_paths` resolves
    /// the target spec under `<planning_root>/spec/apps/<app_key>/specs/`.
    fn scaffold_full_consensus_run(planning_root: &Path) -> PathBuf {
        // repo_root must canonicalize to a real path so the fingerprint matches.
        let repo_root = planning_root.join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        let canonical_repo = repo_root.canonicalize().unwrap();
        let app_key = build_app_key(&canonical_repo, ".");
        let spec_id = "spec-1";

        let run_dir = planning_root.join("plan").join("runs").join("run-1");
        std::fs::create_dir_all(&run_dir).unwrap();

        let run = MigratePlanRunRecord {
            schema_version: MIGRATE_PLAN_RUN_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: app_key.clone(),
            app_path: ".".to_string(),
            repo_root: canonical_repo.display().to_string(),
            remote_repo: None,
            r#ref: None,
            owned_app_keys: Some(vec![app_key.clone()]),
            mode: PlanningMode::FullConsensus,
            stage_order: FULL_CONSENSUS_STAGE_ORDER
                .iter()
                .map(|s| s.to_string())
                .collect(),
            state: RunState::Running,
            escalation_reason: EscalationReason::HardStop,
            confidence: Some("0.91".to_string()),
            hard_stop_reasons: vec!["wide diff".to_string()],
            workspace_scope: WorkspaceScope {
                scope_type: "repo".to_string(),
                marker_path: None,
                workspace_root: Some(canonical_repo.display().to_string()),
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
            repo_fingerprint: sha256_hex(&canonical_repo.display().to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let run_json = run_dir.join("run.json");
        write_json_atomically(&run_json, &serde_json::to_value(&run).unwrap()).unwrap();

        let approval = MigratePlanApprovalRecord {
            schema_version: MIGRATE_PLAN_APPROVAL_SCHEMA_VERSION.to_string(),
            run_id: "run-1".to_string(),
            app_key: app_key.clone(),
            state: ApprovalState::NeedsRevision,
            required_before_execution: true,
            target_spec_id: spec_id.to_string(),
            target_spec_sha256: "sha".to_string(),
            approved_stage_artifacts: vec![],
            adr_sha256: None,
            requested_at: "2026-01-01T00:00:00Z".to_string(),
            approved_at: None,
            approved_by: None,
            approval_prompt_sha256: "prompt".to_string(),
        };
        write_json_atomically(
            &run_dir.join("approval.json"),
            &serde_json::to_value(&approval).unwrap(),
        )
        .unwrap();

        // Target spec markdown + meta must exist: when approval stays non-pending,
        // migrate_stage_write asserts the spec markdown is present.
        let (spec_md, spec_meta, _) =
            spec_artifact_paths(&run_dir, &app_key, spec_id).unwrap();
        write_text_atomically(&spec_md, "# target spec").unwrap();
        write_json_atomically(
            &spec_meta,
            &json!({"status": "draft", "approval": {"state": "needs_revision"}}),
        )
        .unwrap();

        run_json
    }

    #[test]
    fn stage_write_uses_fixed_ordinal_and_overwrites() {
        let temp = tempfile::tempdir().unwrap();
        let run_json = scaffold_full_consensus_run(temp.path());
        let run_dir = run_json.parent().unwrap().to_path_buf();
        let stages_dir = run_dir.join("stages");

        // Draft lives OUTSIDE the run_dir (a separate temp dir).
        let draft_dir = tempfile::tempdir().unwrap();
        let draft = draft_dir.path().join("planner-draft.md");

        std::fs::write(&draft, "# planner v1").unwrap();
        migrate_stage_write(&run_json, "planner", &draft, None, None, None).unwrap();

        std::fs::write(&draft, "# planner v2 (revision)").unwrap();
        migrate_stage_write(&run_json, "planner", &draft, None, None, None).unwrap();

        // Exactly one stage .md file, deterministically named 02-planner.md.
        let md_files: Vec<_> = std::fs::read_dir(&stages_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".md"))
            .collect();
        assert_eq!(md_files, vec!["02-planner.md".to_string()]);

        let planner_md = stages_dir.join("02-planner.md");
        assert_eq!(
            std::fs::read_to_string(&planner_md).unwrap(),
            "# planner v2 (revision)"
        );

        // meta revision is bumped to 2 on the second write.
        let meta: Value = read_json(&stages_dir.join("02-planner.meta.json")).unwrap();
        assert_eq!(meta["revision"].as_u64(), Some(2));
    }

    #[test]
    fn stage_write_rejects_markdown_inside_stages_dir() {
        let temp = tempfile::tempdir().unwrap();
        let run_json = scaffold_full_consensus_run(temp.path());
        let run_dir = run_json.parent().unwrap().to_path_buf();
        let stages_dir = run_dir.join("stages");
        std::fs::create_dir_all(&stages_dir).unwrap();

        // An agent-authored draft placed directly under stages/ must be refused.
        let inside = stages_dir.join("99-manual.md");
        std::fs::write(&inside, "# manual draft").unwrap();

        let err = migrate_stage_write(&run_json, "planner", &inside, None, None, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("stages/"),
            "expected error to mention stages/, got: {err}"
        );
    }

    #[test]
    fn stage_write_redacts_secret_values_on_disk() {
        let temp = tempfile::tempdir().unwrap();
        let run_json = scaffold_full_consensus_run(temp.path());
        let run_dir = run_json.parent().unwrap().to_path_buf();
        let stages_dir = run_dir.join("stages");

        let draft_dir = tempfile::tempdir().unwrap();
        let draft = draft_dir.path().join("discover-draft.md");
        std::fs::write(
            &draft,
            "SLACK_BOT_TOKEN=xoxb-1073512345678-abcDEF123ghi 가 노출됐어요",
        )
        .unwrap();

        migrate_stage_write(&run_json, "discover", &draft, None, None, None).unwrap();

        let discover_md = stages_dir.join("01-discover.md");
        let written = std::fs::read_to_string(&discover_md).unwrap();
        assert!(
            !written.contains("xoxb-1073512345678"),
            "secret leaked to disk: {written}"
        );
        assert!(
            written.contains("<REDACTED_SLACK_TOKEN>"),
            "redaction marker missing: {written}"
        );

        let meta: Value = read_json(&stages_dir.join("01-discover.meta.json")).unwrap();
        assert_eq!(meta["redacted"].as_bool(), Some(true));
    }
}

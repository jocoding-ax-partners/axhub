use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Serialize;

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::atomic_jsonl;
use crate::migrate_planning::{
    build_planning_preview, PlanningMode, PlanningPreview, FULL_CONSENSUS_STAGE_ORDER,
};

const MAX_SCAN_DEPTH: usize = 5;
const MAX_SCAN_FILES: usize = 5_000;
const MAX_SCAN_BYTES: u64 = 50 * 1024 * 1024;
const MAX_SOURCE_FILE_BYTES: u64 = 1024 * 1024;
const COMPOSE_FILES: [&str; 4] = [
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yaml",
    "compose.yml",
];

#[derive(Debug, Serialize)]
pub struct MigratePlanOutput {
    pub schema_version: String,
    pub root: String,
    pub source_dir: String,
    pub monorepo: bool,
    pub candidates: Vec<AppCandidate>,
    pub container_contracts: ContainerContracts,
    pub env_refs: Vec<EnvRef>,
    pub suggested_manifest: String,
    pub sdk_conversion: SdkConversionPlan,
    pub planning: Option<PlanningPreview>,
    pub planning_persistence: Option<PlanningPersistenceOutput>,
}

#[derive(Debug, Serialize)]
pub struct PlanningPersistenceOutput {
    pub schema_version: String,
    pub persisted: bool,
    pub mode: PlanningMode,
    pub reason: String,
    pub run_id: Option<String>,
    pub spec_id: Option<String>,
    pub run_state: Option<String>,
    pub approval_state: Option<String>,
    pub wrote_latest_pointer: bool,
    pub next_required_stages: Vec<String>,
    pub paths: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct AppCandidate {
    pub path: String,
    pub stack_hint: String,
    pub has_dockerfile: bool,
    pub has_compose: bool,
    pub compose_file: Option<String>,
    pub env_refs: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct ContainerContracts {
    pub dockerfile: bool,
    pub compose: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct EnvRef {
    pub name: String,
    pub scope: String,
}

#[derive(Debug, Serialize)]
pub struct SdkConversionPlan {
    pub schema_version: String,
    pub candidates: Vec<SdkConversionCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SdkConversionCandidate {
    pub path: String,
    pub language: String,
    pub framework: String,
    pub dependency_hint: String,
    pub wrapper_target_path: String,
    pub wrapper_preview: String,
    pub env_refs: Vec<EnvRef>,
    pub data_candidates: Vec<SdkSignal>,
    pub auth_candidates: Vec<SdkSignal>,
    pub risk: String,
    pub expected_diff_summary: Vec<String>,
    pub hard_stop_reasons: Vec<String>,
    /// Per-reason override policy (devex-D1=C). Parallel to `hard_stop_reasons`
    /// (which stays byte-identical for existing consumers); classifies each
    /// reason as overridable or absolute.
    pub hard_stop_policy: Vec<HardStopPolicy>,
    /// True when any absolute (non-overridable) reason is present. The SKILL
    /// MUST keep the conversion plan-only — there is no execute path to override.
    pub plan_only: bool,
}

/// Override policy for one hard-stop reason. Absolute reasons (secret exposure,
/// custom/unclear auth, unsupported language) are never overridable: a git
/// rollback cannot un-leak a secret, so the conversion stays structurally
/// plan-only rather than relying on the agent to decline.
#[derive(Debug, Clone, Serialize)]
pub struct HardStopPolicy {
    pub code: String,
    pub message: String,
    pub overridable: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct SdkSignal {
    pub file: String,
    pub reason: String,
}

pub fn run_migrate_plan(args: &[String]) -> Result<i32> {
    let mut dir: Option<PathBuf> = None;
    let mut app_path: Option<String> = None;
    let mut json = false;
    let mut persist_planning = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                json = true;
                index += 1;
            }
            "--persist-planning" => {
                persist_planning = true;
                index += 1;
            }
            "--dir" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-plan: --dir 값이 필요해요");
                };
                dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--app-path" => {
                let Some(value) = args.get(index + 1) else {
                    bail!("migrate-plan: --app-path 값이 필요해요");
                };
                app_path = Some(value.clone());
                index += 2;
            }
            _ => bail!("migrate-plan: unknown option"),
        }
    }
    let dir = dir.unwrap_or(std::env::current_dir()?);
    let output = build_migrate_plan_with_options(&dir, app_path.as_deref(), persist_planning)?;
    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!(
            "migrate 후보 {}개를 찾았어요. --json 으로 상세 계획을 확인해요.",
            output.candidates.len()
        );
    }
    Ok(0)
}

pub fn build_migrate_plan(dir: &Path) -> Result<MigratePlanOutput> {
    build_migrate_plan_with_options(dir, None, false)
}

pub fn build_migrate_plan_with_selection(
    dir: &Path,
    selected_app_path: Option<&str>,
) -> Result<MigratePlanOutput> {
    build_migrate_plan_with_options(dir, selected_app_path, false)
}

fn build_migrate_plan_with_options(
    dir: &Path,
    selected_app_path: Option<&str>,
    persist_planning: bool,
) -> Result<MigratePlanOutput> {
    let root =
        fs::canonicalize(dir).with_context(|| format!("{} 경로를 읽지 못했어요", dir.display()))?;
    if !root.is_dir() {
        bail!("{} 는 디렉터리가 아니에요", root.display());
    }
    let mut scan = ScanState::default();
    collect_files(&root, &root, 0, &mut scan)?;
    let files = scan.files;
    let scanned_env_refs = scan_env_refs(&root, &files);
    let env_refs = unique_env_refs(&scanned_env_refs);
    let candidates = detect_candidates(&root, &files, &scanned_env_refs);
    let container_contracts = ContainerContracts {
        dockerfile: candidates.iter().any(|c| c.has_dockerfile),
        compose: candidates.iter().any(|c| c.has_compose),
    };
    let selected_path = selected_app_path
        .map(normalize_selected_app_path)
        .transpose()?;
    let selected_candidate = match selected_path.as_deref() {
        Some(path) => {
            let Some(candidate) = candidates.iter().find(|candidate| candidate.path == path) else {
                bail!("migrate-plan: 선택한 앱 경로가 후보에 없어요");
            };
            Some(candidate)
        }
        None => candidates.first(),
    };
    let suggested_manifest = render_manifest(
        selected_candidate,
        &manifest_env_refs(selected_candidate, &env_refs),
    );
    let sdk_conversion = build_sdk_conversion(&root, &files, &candidates, &scanned_env_refs);
    let planning = if let Some(candidate) = selected_candidate {
        let hard_stop_reasons = sdk_conversion
            .candidates
            .iter()
            .find(|sdk_candidate| sdk_candidate.path == candidate.path)
            .map(|sdk_candidate| sdk_candidate.hard_stop_reasons.clone())
            .unwrap_or_default();
        build_planning_preview(
            &root,
            Some(candidate.path.as_str()),
            candidates.len(),
            selected_path.is_some() || candidates.len() <= 1,
            Some(candidate.confidence),
            &hard_stop_reasons,
        )?
    } else {
        None
    };
    let planning_persistence = if persist_planning {
        if candidates.len() > 1 && selected_path.is_none() {
            bail!("migrate-plan: planning persistence 는 다중 후보에서 --app-path 가 필요해요");
        }
        persist_migrate_planning(
            &root,
            selected_candidate,
            &env_refs,
            &suggested_manifest,
            &sdk_conversion,
            planning.as_ref(),
        )?
    } else {
        None
    };
    Ok(MigratePlanOutput {
        schema_version: "migrate-plan/v1".to_string(),
        root: root.display().to_string(),
        source_dir: root.display().to_string(),
        monorepo: candidates.len() > 1,
        candidates,
        container_contracts,
        env_refs,
        suggested_manifest,
        sdk_conversion,
        planning,
        planning_persistence,
    })
}

#[derive(Debug, Clone)]
struct SourceScanFile {
    rel_path: PathBuf,
    body: String,
}

fn normalize_selected_app_path(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "." {
        return Ok(".".to_string());
    }
    let replaced = trimmed.replace('\\', "/");
    if replaced.starts_with('/') || replaced.contains(':') || replaced.contains('\0') {
        bail!("migrate-plan: 선택한 앱 경로가 안전하지 않아요");
    }
    let normalized = replaced.trim_end_matches('/');
    if normalized
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        bail!("migrate-plan: 선택한 앱 경로가 안전하지 않아요");
    }
    Ok(normalized.to_string())
}

#[derive(Default)]
struct ScanState {
    files: Vec<PathBuf>,
    total_bytes: u64,
}

#[derive(Debug, Clone)]
struct ScannedEnvRef {
    rel_path: PathBuf,
    env: EnvRef,
}

fn collect_files(root: &Path, dir: &Path, depth: usize, out: &mut ScanState) -> Result<()> {
    if depth > MAX_SCAN_DEPTH
        || out.files.len() >= MAX_SCAN_FILES
        || out.total_bytes >= MAX_SCAN_BYTES
    {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("{} 를 읽지 못했어요", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            ".git" | "node_modules" | "target" | "dist" | "build" | ".next" | "vendor"
        ) {
            continue;
        }
        if metadata.is_dir() {
            let Ok(canonical) = fs::canonicalize(&path) else {
                continue;
            };
            if !canonical.starts_with(root) {
                continue;
            }
            collect_files(root, &canonical, depth + 1, out)?;
        } else if metadata.is_file() {
            if out.files.len() >= MAX_SCAN_FILES || metadata.len() > MAX_SOURCE_FILE_BYTES {
                continue;
            }
            let Some(next_total) = out.total_bytes.checked_add(metadata.len()) else {
                break;
            };
            if next_total > MAX_SCAN_BYTES {
                break;
            }
            let Ok(canonical) = fs::canonicalize(&path) else {
                continue;
            };
            if !canonical.starts_with(root) {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(root) {
                out.files.push(rel.to_path_buf());
                out.total_bytes = next_total;
            }
        }
    }
    Ok(())
}

fn detect_candidates(
    root: &Path,
    files: &[PathBuf],
    env_refs: &[ScannedEnvRef],
) -> Vec<AppCandidate> {
    let mut by_dir: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    for rel in files {
        let Some(file) = rel.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let dir = rel.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        if stack_for_marker(file).is_some() || file == "Dockerfile" || COMPOSE_FILES.contains(&file)
        {
            by_dir.entry(dir).or_default().insert(file.to_string());
        }
    }
    let mut out = Vec::new();
    for (dir, markers) in by_dir {
        let stack = markers
            .iter()
            .find_map(|m| stack_for_marker(m))
            .unwrap_or("container");
        let has_dockerfile = markers.contains("Dockerfile");
        let compose_file = COMPOSE_FILES
            .into_iter()
            .find(|name| markers.contains(*name))
            .map(str::to_string);
        let has_compose = compose_file.is_some();
        let path = path_to_portable_json(&dir);
        let confidence = if has_dockerfile {
            0.95
        } else if compose_file.is_some() {
            0.85
        } else {
            0.80
        };
        out.push(AppCandidate {
            path,
            stack_hint: stack.to_string(),
            has_dockerfile,
            has_compose,
            compose_file,
            env_refs: env_refs_for_candidate(&dir, env_refs),
            confidence,
        });
    }
    if out.is_empty() && has_regular_file(&root.join("Dockerfile")) {
        out.push(AppCandidate {
            path: ".".to_string(),
            stack_hint: "container".to_string(),
            has_dockerfile: true,
            has_compose: false,
            compose_file: None,
            env_refs: env_refs_for_candidate(Path::new("."), env_refs),
            confidence: 0.95,
        });
    }
    out
}

fn has_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_file())
        .unwrap_or(false)
}

fn stack_for_marker(file: &str) -> Option<&'static str> {
    match file {
        "package.json" => Some("node"),
        "requirements.txt" | "pyproject.toml" => Some("python"),
        "go.mod" => Some("go"),
        "Cargo.toml" => Some("rust"),
        "Gemfile" => Some("ruby"),
        "pom.xml" | "build.gradle" => Some("java"),
        "build.gradle.kts" | "settings.gradle.kts" => Some("kotlin"),
        _ => None,
    }
}

fn scan_env_refs(root: &Path, files: &[PathBuf]) -> Vec<ScannedEnvRef> {
    let patterns = [
        Regex::new(r#"process\.env\.([A-Z_][A-Z0-9_]*)"#).unwrap(),
        Regex::new(r#"process\.env\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(),
        Regex::new(r#"os\.environ(?:\.get)?\(["']([A-Z_][A-Z0-9_]*)["']"#).unwrap(),
        Regex::new(r#"os\.environ\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(),
        Regex::new(r#"ENV\[["']([A-Z_][A-Z0-9_]*)["']\]"#).unwrap(),
    ];
    let mut refs = BTreeSet::new();
    for rel in files {
        if !is_source_file(rel) {
            continue;
        }
        let path = root.join(rel);
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_SOURCE_FILE_BYTES {
            continue;
        }
        let Ok(body) = fs::read_to_string(path) else {
            continue;
        };
        for re in &patterns {
            for cap in re.captures_iter(&body) {
                let name = cap[1].to_string();
                refs.insert((
                    rel.clone(),
                    EnvRef {
                        scope: scope_for_env(&name).to_string(),
                        name,
                    },
                ));
            }
        }
    }
    refs.into_iter()
        .map(|(rel_path, env)| ScannedEnvRef { rel_path, env })
        .collect()
}

fn unique_env_refs(scanned: &[ScannedEnvRef]) -> Vec<EnvRef> {
    scanned
        .iter()
        .map(|entry| entry.env.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn env_refs_for_candidate(dir: &Path, env_refs: &[ScannedEnvRef]) -> Vec<String> {
    let names = env_refs
        .iter()
        .filter(|entry| rel_belongs_to_candidate(&entry.rel_path, dir))
        .map(|entry| entry.env.name.clone())
        .collect::<BTreeSet<_>>();
    names.into_iter().collect()
}

fn manifest_env_refs(candidate: Option<&AppCandidate>, env_refs: &[EnvRef]) -> Vec<EnvRef> {
    let Some(candidate) = candidate else {
        return Vec::new();
    };
    let candidate_names = candidate.env_refs.iter().collect::<BTreeSet<_>>();
    env_refs
        .iter()
        .filter(|entry| candidate_names.contains(&entry.name))
        .cloned()
        .collect()
}

fn rel_belongs_to_candidate(rel: &Path, dir: &Path) -> bool {
    dir.as_os_str().is_empty() || dir == Path::new(".") || rel.starts_with(dir)
}

fn path_to_portable_json(path: &Path) -> String {
    if path.as_os_str().is_empty() || path == Path::new(".") {
        ".".to_string()
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("js" | "jsx" | "ts" | "tsx" | "py" | "rb" | "go" | "java" | "kt" | "kts")
    )
}

fn scope_for_env(name: &str) -> &'static str {
    if name.starts_with("NEXT_PUBLIC_") || name.starts_with("VITE_") || name.starts_with("PUBLIC_")
    {
        "build"
    } else {
        "runtime"
    }
}

fn build_sdk_conversion(
    root: &Path,
    files: &[PathBuf],
    candidates: &[AppCandidate],
    scanned_env_refs: &[ScannedEnvRef],
) -> SdkConversionPlan {
    SdkConversionPlan {
        schema_version: "sdk-conversion/v1".to_string(),
        candidates: candidates
            .iter()
            .map(|candidate| build_sdk_candidate(root, files, candidate, scanned_env_refs))
            .collect(),
    }
}

fn build_sdk_candidate(
    root: &Path,
    files: &[PathBuf],
    candidate: &AppCandidate,
    scanned_env_refs: &[ScannedEnvRef],
) -> SdkConversionCandidate {
    let dir = candidate_dir(&candidate.path);
    let source_files = source_files_for_candidate(root, files, &dir);
    let env_refs = sdk_env_refs_for_candidate(&dir, scanned_env_refs);
    let language = candidate.stack_hint.clone();
    let framework = detect_framework_hint(root, &dir, &language, &source_files);
    let dependency_hint = dependency_hint_for_candidate(root, &dir, &language);
    let wrapper_target_path = wrapper_target_path(&candidate.path, root, &dir, &language);
    let wrapper_preview = render_wrapper_preview(&language, &framework, &source_files);
    let data_candidates = detect_data_candidates(&language, &source_files);
    let auth_candidates = detect_auth_candidates(&language, &source_files);
    let hard_stop_policy = detect_hard_stop_reasons(
        root,
        files,
        candidate,
        &dir,
        &language,
        &env_refs,
        &data_candidates,
        &auth_candidates,
        &source_files,
    );
    let hard_stop_reasons: Vec<String> =
        hard_stop_policy.iter().map(|p| p.message.clone()).collect();
    // Absolute (non-overridable) reason present → conversion is structurally plan-only.
    let plan_only = hard_stop_policy.iter().any(|p| !p.overridable);
    let risk = if !hard_stop_reasons.is_empty() {
        "high"
    } else if !data_candidates.is_empty() || !auth_candidates.is_empty() {
        "medium"
    } else {
        "low"
    }
    .to_string();
    let mut expected_diff_summary = Vec::new();
    if !dependency_hint.is_empty() {
        expected_diff_summary.push(format!("dependency hint: {dependency_hint}"));
    }
    if !wrapper_target_path.is_empty() {
        expected_diff_summary.push(format!("wrapper target: {wrapper_target_path}"));
    }
    if !wrapper_preview.is_empty() {
        expected_diff_summary.push("wrapper preview is deterministic and reversible".to_string());
    }
    if !env_refs.is_empty() {
        expected_diff_summary.push(format!(
            "map {} env ref(s) into wrapper config",
            env_refs.len()
        ));
    }
    if !data_candidates.is_empty() {
        expected_diff_summary.push(format!(
            "plan-only review for {} data candidate file(s)",
            data_candidates.len()
        ));
    }
    if !auth_candidates.is_empty() {
        expected_diff_summary.push(format!(
            "plan-only review for {} auth candidate file(s)",
            auth_candidates.len()
        ));
    }
    if !hard_stop_reasons.is_empty() {
        expected_diff_summary
            .push("preview only: hard-stop reasons block automatic patching".to_string());
    }
    SdkConversionCandidate {
        path: candidate.path.clone(),
        language,
        framework,
        dependency_hint,
        wrapper_target_path,
        wrapper_preview,
        env_refs,
        data_candidates,
        auth_candidates,
        risk,
        expected_diff_summary,
        hard_stop_reasons,
        hard_stop_policy,
        plan_only,
    }
}

fn candidate_dir(path: &str) -> PathBuf {
    if path.is_empty() || path == "." {
        PathBuf::from(".")
    } else {
        PathBuf::from(path)
    }
}

fn source_files_for_candidate(root: &Path, files: &[PathBuf], dir: &Path) -> Vec<SourceScanFile> {
    let mut out = Vec::new();
    for rel in files {
        if !rel_belongs_to_candidate(rel, dir) || !is_source_file(rel) {
            continue;
        }
        let path = root.join(rel);
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_SOURCE_FILE_BYTES {
            continue;
        }
        let Ok(body) = fs::read_to_string(path) else {
            continue;
        };
        out.push(SourceScanFile {
            rel_path: rel.clone(),
            body,
        });
    }
    out.sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    out
}

fn sdk_env_refs_for_candidate(dir: &Path, env_refs: &[ScannedEnvRef]) -> Vec<EnvRef> {
    env_refs
        .iter()
        .filter(|entry| rel_belongs_to_candidate(&entry.rel_path, dir))
        .map(|entry| entry.env.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn detect_framework_hint(
    root: &Path,
    dir: &Path,
    language: &str,
    source_files: &[SourceScanFile],
) -> String {
    match language {
        "node" => detect_node_framework(root, dir),
        "python" => detect_python_framework(root, dir),
        "go" => detect_source_framework(
            source_files,
            &[
                ("gin", "gin"),
                ("fiber", "fiber"),
                ("echo", "echo"),
                ("chi", "chi"),
            ],
            "go",
        ),
        "ruby" => detect_ruby_framework(root, dir, source_files),
        "java" => detect_source_framework(source_files, &[("spring", "spring")], "java"),
        "kotlin" => detect_source_framework(
            source_files,
            &[("ktor", "ktor"), ("spring", "spring")],
            "kotlin",
        ),
        other => other.to_string(),
    }
}

fn detect_node_framework(root: &Path, dir: &Path) -> String {
    let Some(text) = read_candidate_text(root, dir, &["package.json"]) else {
        return "node".to_string();
    };
    let lower = text.to_lowercase();
    for (needle, framework) in [
        ("\"next\"", "nextjs"),
        ("\"nuxt\"", "nuxt"),
        ("\"sveltekit\"", "sveltekit"),
        ("\"@remix-run", "remix"),
        ("\"vite\"", "vite"),
    ] {
        if lower.contains(needle) {
            return framework.to_string();
        }
    }
    "node".to_string()
}

fn detect_python_framework(root: &Path, dir: &Path) -> String {
    let Some(text) = read_candidate_text(root, dir, &["pyproject.toml", "requirements.txt"]) else {
        return "python".to_string();
    };
    let lower = text.to_lowercase();
    for (needle, framework) in [
        ("fastapi", "fastapi"),
        ("django", "django"),
        ("flask", "flask"),
    ] {
        if lower.contains(needle) {
            return framework.to_string();
        }
    }
    "python".to_string()
}

fn detect_ruby_framework(root: &Path, dir: &Path, source_files: &[SourceScanFile]) -> String {
    if let Some(text) = read_candidate_text(root, dir, &["Gemfile"]) {
        let lower = text.to_lowercase();
        if lower.contains("rails") {
            return "rails".to_string();
        }
        if lower.contains("sinatra") {
            return "sinatra".to_string();
        }
    }
    detect_source_framework(
        source_files,
        &[("rails", "rails"), ("sinatra", "sinatra")],
        "ruby",
    )
}

fn detect_source_framework(
    source_files: &[SourceScanFile],
    patterns: &[(&str, &str)],
    fallback: &str,
) -> String {
    for source in source_files {
        let lower = source.body.to_lowercase();
        for (needle, framework) in patterns {
            if lower.contains(needle) {
                return (*framework).to_string();
            }
        }
    }
    fallback.to_string()
}

fn dependency_hint_for_candidate(root: &Path, dir: &Path, language: &str) -> String {
    match language {
        "node" => {
            if root.join(dir).join("pnpm-lock.yaml").is_file() {
                "pnpm add @ax-hub/sdk".to_string()
            } else if root.join(dir).join("yarn.lock").is_file() {
                "yarn add @ax-hub/sdk".to_string()
            } else if root.join(dir).join("bun.lock").is_file()
                || root.join(dir).join("bun.lockb").is_file()
            {
                "bun add @ax-hub/sdk".to_string()
            } else {
                "npm install @ax-hub/sdk".to_string()
            }
        }
        "python" => "pip install axhub-sdk".to_string(),
        "go" => "go get github.com/jocoding-ax-partners/axhub-sdk-go".to_string(),
        "ruby" => "bundle add axhub-sdk".to_string(),
        "java" => "implementation(\"ai.axhub:axhub-sdk-java:<version>\")".to_string(),
        "kotlin" => "implementation(\"ai.axhub:axhub-sdk-kotlin:<version>\")".to_string(),
        "rust" => {
            "manual review required: no axhub rust SDK wrapper contract in this lane".to_string()
        }
        _ => {
            "manual review required: framework markers are insufficient for an SDK hint".to_string()
        }
    }
}

fn wrapper_target_path(candidate_path: &str, root: &Path, dir: &Path, language: &str) -> String {
    let relative = match language {
        "node" => {
            if root.join(dir).join("src").is_dir() {
                "src/axhub.ts"
            } else {
                "axhub.ts"
            }
        }
        "python" => {
            if root.join(dir).join("src").is_dir() {
                "src/axhub_client.py"
            } else {
                "axhub_client.py"
            }
        }
        "go" => "axhub_client.go",
        "ruby" => {
            if root.join(dir).join("lib").is_dir() {
                "lib/axhub_client.rb"
            } else {
                "axhub_client.rb"
            }
        }
        "java" => "src/main/java/ai/axhub/sdk/AxhubClientFactory.java",
        "kotlin" => "src/main/kotlin/ai/axhub/sdk/AxhubClientFactory.kt",
        _ => "manual-review-only",
    };
    candidate_file_path(candidate_path, relative)
}

fn render_wrapper_preview(
    language: &str,
    framework: &str,
    source_files: &[SourceScanFile],
) -> String {
    match language {
        "node" => format!(
            "import {{ AxHubClient }} from '@ax-hub/sdk';\n\n// framework: {framework}\nexport const axhub = new AxHubClient({{\n  token: process.env.AX_HUB_PAT!,\n  tokenType: 'pat',\n  defaultTenantId: process.env.AX_HUB_TENANT_ID,\n  defaultTenantSlug: process.env.AX_HUB_TENANT_SLUG,\n}});\n"
        ),
        "python" => "import os\nfrom axhub_sdk import AxHubClient, TokenType\n\n\ndef build_axhub_client() -> AxHubClient:\n    return AxHubClient(\n        base_url=\"https://api.axhub.ai\",\n        token=os.environ[\"AXHUB_TOKEN\"],\n        token_type=TokenType.PAT,\n        default_tenant_id=os.environ[\"AXHUB_TENANT_ID\"],\n        default_tenant_slug=os.environ.get(\"AXHUB_TENANT_SLUG\", \"test\"),\n    )\n"
            .to_string(),
        "go" => format!(
            "package {}\n\nimport (\n    \"os\"\n\n    axhub \"github.com/jocoding-ax-partners/axhub-sdk-go\"\n)\n\nfunc NewAxHubClient() *axhub.Client {{\n    return axhub.NewClient(axhub.Config{{\n        BaseURL:           \"https://api.axhub.ai\",\n        Token:             os.Getenv(\"AXHUB_TOKEN\"),\n        TokenType:         axhub.TokenTypePAT,\n        DefaultTenantID:   os.Getenv(\"AXHUB_TENANT_ID\"),\n        DefaultTenantSlug: os.Getenv(\"AXHUB_TENANT_SLUG\"),\n    }})\n}}\n",
            detect_go_package(source_files).as_deref().unwrap_or("main")
        ),
        "ruby" => "require 'axhub_sdk'\n\nAXHUB_CLIENT = AxHub::Client.new(\n  base_url: 'https://api.axhub.ai',\n  token: ENV.fetch('AXHUB_TOKEN'),\n  token_type: :pat,\n  default_tenant_id: ENV.fetch('AXHUB_TENANT_ID'),\n  default_tenant_slug: ENV.fetch('AXHUB_TENANT_SLUG', 'test'),\n)\n"
            .to_string(),
        "java" => format!(
            "package {};\n\nimport ai.axhub.sdk.AxHubClient;\nimport ai.axhub.sdk.TokenType;\n\npublic final class AxhubClientFactory {{\n    private AxhubClientFactory() {{}}\n\n    public static AxHubClient create() {{\n        return AxHubClient.builder()\n            .baseUrl(\"https://api.axhub.ai\")\n            .token(System.getenv(\"AXHUB_TOKEN\"))\n            .tokenType(TokenType.PAT)\n            .defaultTenantId(System.getenv(\"AXHUB_TENANT_ID\"))\n            .build();\n    }}\n}}\n",
            detect_jvm_package(source_files).as_deref().unwrap_or("ai.axhub.sdk")
        ),
        "kotlin" => format!(
            "package {}\n\nimport ai.axhub.sdk.AxHubKotlinClient\nimport ai.axhub.sdk.TokenType\n\nfun buildAxhubClient(): AxHubKotlinClient =\n    AxHubKotlinClient(\n        baseUrl = \"https://api.axhub.ai\",\n        token = System.getenv(\"AXHUB_TOKEN\"),\n        tokenType = TokenType.PAT,\n        defaultTenantId = System.getenv(\"AXHUB_TENANT_ID\"),\n    )\n",
            detect_jvm_package(source_files).as_deref().unwrap_or("ai.axhub.sdk")
        ),
        _ => "// manual review required before generating an SDK wrapper for this language\n".to_string(),
    }
}

fn detect_go_package(source_files: &[SourceScanFile]) -> Option<String> {
    let re = Regex::new(r"(?m)^package\s+([A-Za-z_][A-Za-z0-9_]*)$").unwrap();
    source_files.iter().find_map(|source| {
        re.captures(&source.body)
            .and_then(|caps| caps.get(1).map(|value| value.as_str().to_string()))
    })
}

fn detect_jvm_package(source_files: &[SourceScanFile]) -> Option<String> {
    let re = Regex::new(r"(?m)^\s*package\s+([A-Za-z0-9_.]+)\s*;?").unwrap();
    source_files.iter().find_map(|source| {
        re.captures(&source.body)
            .and_then(|caps| caps.get(1).map(|value| value.as_str().to_string()))
    })
}

fn detect_data_candidates(language: &str, source_files: &[SourceScanFile]) -> Vec<SdkSignal> {
    let patterns: &[(&str, &str)] = match language {
        "node" => &[
            ("prisma", "ORM/DB access candidate"),
            ("sequelize", "ORM/DB access candidate"),
            ("knex", "query builder candidate"),
            ("db.query", "raw query candidate"),
            ("pool.query", "raw query candidate"),
            ("fetch(", "HTTP data fetch candidate"),
            ("axios.", "HTTP data fetch candidate"),
        ],
        "python" => &[
            ("sqlalchemy", "ORM/DB access candidate"),
            ("psycopg", "driver-level data access candidate"),
            ("cursor.execute", "raw query candidate"),
            ("requests.", "HTTP data fetch candidate"),
            ("httpx.", "HTTP data fetch candidate"),
        ],
        "go" => &[
            ("database/sql", "database/sql candidate"),
            ("gorm", "ORM/DB access candidate"),
            ("sqlx", "sqlx data candidate"),
            (".query(", "raw query candidate"),
            (".exec(", "raw query candidate"),
        ],
        "ruby" => &[
            ("activerecord", "ORM/DB access candidate"),
            ("sequel", "ORM/DB access candidate"),
            ("pg.connect", "driver-level data access candidate"),
            ("faraday", "HTTP data fetch candidate"),
            ("net::http", "HTTP data fetch candidate"),
        ],
        "java" | "kotlin" => &[
            ("jdbctemplate", "JDBC data access candidate"),
            ("entitymanager", "ORM/DB access candidate"),
            ("webclient", "HTTP data fetch candidate"),
            ("resttemplate", "HTTP data fetch candidate"),
            ("httpclient", "HTTP data fetch candidate"),
        ],
        _ => &[],
    };
    detect_signals(source_files, patterns)
}

fn detect_auth_candidates(language: &str, source_files: &[SourceScanFile]) -> Vec<SdkSignal> {
    let patterns: &[(&str, &str)] = match language {
        "node" => &[
            ("next-auth", "NextAuth/Auth.js auth candidate"),
            ("nextauth", "NextAuth/Auth.js auth candidate"),
            ("passport", "Passport auth candidate"),
            ("clerk", "Clerk auth candidate"),
            ("supabase.auth", "Supabase auth candidate"),
            ("auth0", "Auth0 auth candidate"),
            ("jwt", "JWT/session auth candidate"),
        ],
        "python" => &[
            ("django.contrib.auth", "Django auth candidate"),
            ("flask_login", "Flask-Login auth candidate"),
            ("login_required", "login guard candidate"),
            ("request.user", "request.user boundary candidate"),
            ("jwt", "JWT/session auth candidate"),
        ],
        "go" => &[
            ("oauth2", "OAuth2 auth candidate"),
            ("jwt", "JWT/session auth candidate"),
            ("middleware", "middleware auth candidate"),
        ],
        "ruby" => &[
            ("devise", "Devise auth candidate"),
            ("omniauth", "OmniAuth auth candidate"),
            ("current_user", "current_user boundary candidate"),
            ("authenticate_user!", "auth guard candidate"),
            ("session[", "session auth candidate"),
        ],
        "java" | "kotlin" => &[
            ("spring security", "Spring Security auth candidate"),
            ("securityfilterchain", "security filter auth candidate"),
            ("oauth2", "OAuth2 auth candidate"),
            ("jwt", "JWT/session auth candidate"),
            ("authentication", "authentication boundary candidate"),
            ("ktor.auth", "Ktor auth candidate"),
        ],
        _ => &[],
    };
    let mut out = detect_signals(source_files, patterns);
    for source in source_files {
        let path_lower = path_to_portable_json(&source.rel_path).to_lowercase();
        if ["auth", "login", "logout", "callback", "middleware", "guard"]
            .iter()
            .any(|needle| path_lower.contains(needle))
        {
            let signal = SdkSignal {
                file: path_to_portable_json(&source.rel_path),
                reason: "auth-like file path candidate".to_string(),
            };
            if !out.contains(&signal) {
                out.push(signal);
            }
        }
    }
    out.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.reason.cmp(&right.reason))
    });
    out
}

fn detect_signals(source_files: &[SourceScanFile], patterns: &[(&str, &str)]) -> Vec<SdkSignal> {
    let mut out = BTreeSet::new();
    for source in source_files {
        let lower = source.body.to_lowercase();
        for (needle, reason) in patterns {
            if lower.contains(needle) {
                out.insert(SdkSignal {
                    file: path_to_portable_json(&source.rel_path),
                    reason: (*reason).to_string(),
                });
            }
        }
    }
    out.into_iter().collect()
}

/// Single source of truth for hard-stop override policy (devex-D1=C). Fail-closed:
/// only the three explicitly-listed reasons are overridable; secret_exposure,
/// custom_auth, unsupported_language, and any future/unknown reason default to
/// absolute. A git rollback cannot un-leak a secret, so those stay plan-only.
pub fn hard_stop_reason_overridable(code: &str) -> bool {
    matches!(code, "missing_verification" | "raw_query" | "broad_diff")
}

#[expect(
    clippy::too_many_arguments,
    reason = "The hard-stop classifier intentionally combines multiple explicit signals into one risk gate."
)]
fn detect_hard_stop_reasons(
    root: &Path,
    files: &[PathBuf],
    candidate: &AppCandidate,
    dir: &Path,
    language: &str,
    env_refs: &[EnvRef],
    data_candidates: &[SdkSignal],
    auth_candidates: &[SdkSignal],
    source_files: &[SourceScanFile],
) -> Vec<HardStopPolicy> {
    fn policy(code: &str, message: &str) -> HardStopPolicy {
        HardStopPolicy {
            code: code.to_string(),
            message: message.to_string(),
            overridable: hard_stop_reason_overridable(code),
        }
    }
    let mut policies: Vec<HardStopPolicy> = Vec::new();
    if !matches!(
        language,
        "node" | "python" | "go" | "ruby" | "java" | "kotlin"
    ) {
        // Absolute: no expert/pack exists for this language — cannot be forced.
        policies.push(policy(
            "unsupported_language",
            "지원되는 SDK wrapper 언어 범위 밖이라 manual review 가 필요해요",
        ));
    }
    if !has_verification_anchor(files, dir) {
        // Overridable: a power user can accept the risk; the git checkpoint is the net.
        policies.push(policy(
            "missing_verification",
            "검증 명령 또는 테스트 anchor 를 찾지 못해서 automatic patch 를 막아요",
        ));
    }
    if has_raw_query_signal(data_candidates) {
        policies.push(policy(
            "raw_query",
            "raw query 중심 데이터 접근 가능성이 있어 automatic data patch 를 막아요",
        ));
    }
    if !auth_candidates.is_empty() && !has_known_auth_library(language, source_files) {
        // Absolute: custom/unclear auth — getting it wrong can leak credentials.
        policies.push(policy(
            "custom_auth",
            "auth 구조가 커스텀 또는 불명확해 보여서 auth patch 는 plan-only 예요",
        ));
    }
    if env_refs.iter().any(|env| is_secretish_env(&env.name)) {
        // Absolute: a git rollback cannot un-leak a secret that hit a commit/log.
        policies.push(policy(
            "secret_exposure",
            "secret 성격 env reference 가 보여서 preview-only 로 유지해요",
        ));
    }
    if data_candidates.len() + auth_candidates.len() > 6
        || touches_core_entry(candidate, root, dir, source_files)
    {
        policies.push(policy(
            "broad_diff",
            "wide diff 또는 핵심 진입 파일 blast radius 가능성이 있어 plan-only 로 멈춰요",
        ));
    }
    // Preserve the prior BTreeSet<String> ordering of the derived `hard_stop_reasons`
    // (sorted + deduped by message) so existing consumers stay byte-identical.
    policies.sort_by(|a, b| a.message.cmp(&b.message));
    policies.dedup_by(|a, b| a.message == b.message);
    policies
}

fn has_verification_anchor(files: &[PathBuf], dir: &Path) -> bool {
    files
        .iter()
        .filter(|rel| rel_belongs_to_candidate(rel, dir))
        .any(|rel| {
            let raw = path_to_portable_json(rel).to_lowercase();
            raw.contains("/tests/")
                || raw.contains("/__tests__/")
                || raw.contains("/test/")
                || raw.ends_with("_test.go")
                || raw.ends_with(".spec.ts")
                || raw.ends_with(".spec.js")
                || raw.ends_with(".test.ts")
                || raw.ends_with(".test.js")
                || raw.ends_with("_spec.rb")
                || raw.ends_with("test.rb")
                || raw.contains("pytest")
                || raw.contains("vitest")
                || raw.contains("spec/")
                || raw.contains("src/test/")
        })
}

fn has_raw_query_signal(data_candidates: &[SdkSignal]) -> bool {
    data_candidates
        .iter()
        .any(|candidate| candidate.reason.to_lowercase().contains("raw query"))
}

fn has_known_auth_library(language: &str, source_files: &[SourceScanFile]) -> bool {
    let patterns: &[&str] = match language {
        "node" => &[
            "next-auth",
            "nextauth",
            "passport",
            "clerk",
            "supabase.auth",
            "auth0",
        ],
        "python" => &["django.contrib.auth", "flask_login"],
        "go" => &["oauth2", "jwt"],
        "ruby" => &["devise", "omniauth"],
        "java" | "kotlin" => &["spring security", "oauth2", "ktor.auth"],
        _ => &[],
    };
    source_files.iter().any(|source| {
        let lower = source.body.to_lowercase();
        patterns.iter().any(|needle| lower.contains(needle))
    })
}

fn is_secretish_env(name: &str) -> bool {
    [
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "CLIENT_SECRET",
        "PRIVATE_KEY",
        "API_KEY",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

fn touches_core_entry(
    candidate: &AppCandidate,
    root: &Path,
    _dir: &Path,
    source_files: &[SourceScanFile],
) -> bool {
    let Some(file) = source_files.iter().find(|source| {
        matches!(
            source.rel_path.file_name().and_then(|s| s.to_str()),
            Some(
                "main.ts"
                    | "main.js"
                    | "index.ts"
                    | "index.js"
                    | "main.go"
                    | "app.rb"
                    | "Application.java"
                    | "Application.kt"
                    | "server.ts"
                    | "server.js"
            )
        )
    }) else {
        return false;
    };
    candidate.confidence >= 0.80 && source_files.len() > 1 && root.join(&file.rel_path).exists()
}

fn read_candidate_text(root: &Path, dir: &Path, names: &[&str]) -> Option<String> {
    for name in names {
        let path = root.join(dir).join(name);
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_SOURCE_FILE_BYTES {
            continue;
        }
        if let Ok(text) = fs::read_to_string(path) {
            return Some(text);
        }
    }
    None
}

fn render_manifest(candidate: Option<&AppCandidate>, env_refs: &[EnvRef]) -> String {
    let name = candidate
        .and_then(|c| Path::new(&c.path).file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("migrated-app");
    let mut out = format!(
        "version: axhub/v1\nname: {}\nbuild:\n  strategy: auto\n",
        yaml_double_quote(name)
    );
    if let Some(c) = candidate {
        if c.has_dockerfile && !c.has_compose && c.path != "." {
            out.push_str(&format!(
                "  dockerfile: {}\n",
                yaml_double_quote(&candidate_file_path(&c.path, "Dockerfile"))
            ));
        }
        if let Some(compose) = &c.compose_file {
            out.push_str(&format!(
                "  deploy_method: compose\n  compose_file: {}\n",
                yaml_double_quote(&candidate_file_path(&c.path, compose))
            ));
        }
    }
    if !env_refs.is_empty() {
        out.push_str("env:\n  required:\n");
        for env in env_refs {
            out.push_str(&format!(
                "    - {{ name: {}, scope: {} }}\n",
                yaml_double_quote(&env.name),
                yaml_double_quote(&env.scope)
            ));
        }
    }
    out
}

fn candidate_file_path(candidate_path: &str, file: &str) -> String {
    if candidate_path.is_empty() || candidate_path == "." {
        file.to_string()
    } else {
        format!("{}/{}", candidate_path.trim_end_matches('/'), file)
    }
}

fn yaml_double_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn persist_migrate_planning(
    root: &Path,
    selected_candidate: Option<&AppCandidate>,
    env_refs: &[EnvRef],
    suggested_manifest: &str,
    sdk_conversion: &SdkConversionPlan,
    planning: Option<&PlanningPreview>,
) -> Result<Option<PlanningPersistenceOutput>> {
    let (Some(candidate), Some(planning)) = (selected_candidate, planning) else {
        return Ok(None);
    };
    if planning.mode == PlanningMode::Simple {
        return Ok(Some(PlanningPersistenceOutput {
            schema_version: "migrate-plan-persistence/v1".to_string(),
            persisted: false,
            mode: planning.mode,
            reason: "simple_flow_preserved".to_string(),
            run_id: None,
            spec_id: None,
            run_state: None,
            approval_state: None,
            wrote_latest_pointer: false,
            next_required_stages: vec![],
            paths: BTreeMap::new(),
        }));
    }

    let now = now_ts();
    let planning_root = PathBuf::from(&planning.path_templates.planning_root);
    let run_id = format!(
        "{}-{}",
        now.replace([':', '-'], "")
            .replace('T', "-")
            .replace('Z', "")
            .trim_matches('-'),
        short_sha256(
            &format!("{}:{}:{}", planning.app_key, candidate.path, now),
            6
        )
    );
    let spec_id = format!(
        "{}-{}",
        planning.app_key,
        short_sha256(&format!("{}:{}", run_id, candidate.path), 8)
    );
    let paths = persistence_paths(&planning_root, &planning.app_key, &run_id, &spec_id);

    ensure_repo_fingerprint_matches(paths.latest_run_json.as_path(), &planning.repo_fingerprint)?;
    ensure_repo_fingerprint_matches(paths.run_json.as_path(), &planning.repo_fingerprint)?;

    let selected_sdk = sdk_conversion
        .candidates
        .iter()
        .find(|sdk_candidate| sdk_candidate.path == candidate.path);
    let spec_markdown = render_planning_spec_markdown(
        root,
        candidate,
        env_refs,
        suggested_manifest,
        selected_sdk,
        planning,
    );
    let spec_sha = sha256_hex(&spec_markdown);
    let spec_meta_status = if planning.mode == PlanningMode::SpecOnly {
        "pending_approval"
    } else {
        "draft"
    };
    let approval_state = if planning.mode == PlanningMode::SpecOnly {
        "pending_approval"
    } else {
        "needs_revision"
    };
    let run_state = if planning.mode == PlanningMode::SpecOnly {
        "pending_approval"
    } else {
        "running"
    };

    let spec_meta = json!({
        "schema_version": "axhub/migrate-spec/v1",
        "spec_id": spec_id,
        "app_key": planning.app_key,
        "app_path": candidate.path,
        "repo_root": root.display().to_string(),
        "status": spec_meta_status,
        "mode": planning.mode.as_str(),
        "source_plan_run_id": run_id,
        "candidate_confidence": format!("{:.2}", candidate.confidence),
        "escalation_reason": planning.escalation_reason.as_str(),
        "approval": {
            "state": approval_state,
            "approved_at": Value::Null,
            "approved_by": Value::Null,
            "approval_prompt_sha256": sha256_hex(&format!("{}:{}", planning.app_key, approval_state))
        },
        "artifact_sha256": spec_sha,
        "created_at": now,
        "updated_at": now,
        "redaction": {
            "secret_values": false,
            "raw_prompt_body": false
        }
    });

    let run_json = json!({
        "schema_version": "axhub/migrate-plan-run/v1",
        "run_id": run_id,
        "app_key": planning.app_key,
        "app_path": candidate.path,
        "repo_root": root.display().to_string(),
        "owned_app_keys": [planning.app_key.clone()],
        "mode": planning.mode.as_str(),
        "stage_order": if planning.mode == PlanningMode::FullConsensus {
            FULL_CONSENSUS_STAGE_ORDER.iter().map(|stage| stage.to_string()).collect::<Vec<_>>()
        } else {
            Vec::<String>::new()
        },
        "state": run_state,
        "escalation_reason": planning.escalation_reason.as_str(),
        "confidence": format!("{:.2}", candidate.confidence),
        "hard_stop_reasons": selected_sdk.map(|sdk| sdk.hard_stop_reasons.clone()).unwrap_or_default(),
        "plan_only": selected_sdk.map(|sdk| sdk.plan_only).unwrap_or(false),
        "workspace_scope": planning.workspace_scope,
        "parallelism": {
            "enabled": false,
            "scope": planning.parallelism.scope,
            "reason": planning.parallelism.reason,
            "fallback_reason": planning.parallelism.fallback_reason
        },
        "wave_policy": {
            "mode": planning.wave_policy.mode,
            "independence_required": planning.wave_policy.independence_required,
            "multi_app_allowed": planning.wave_policy.multi_app_allowed
        },
        "wave_index_path": Value::Null,
        "repo_fingerprint": planning.repo_fingerprint,
        "created_at": now,
        "updated_at": now
    });

    let approval_json = json!({
        "schema_version": "axhub/migrate-plan-approval/v1",
        "run_id": run_id,
        "app_key": planning.app_key,
        "state": approval_state,
        "required_before_execution": true,
        "target_spec_id": spec_id,
        "target_spec_sha256": spec_sha,
        "approved_stage_artifacts": Value::Array(vec![]),
        "adr_sha256": Value::Null,
        "requested_at": now,
        "approved_at": Value::Null,
        "approved_by": Value::Null,
        "approval_prompt_sha256": sha256_hex(&format!("{}:{}", planning.app_key, run_state))
    });

    let latest_run_json = json!({
        "schema_version": "axhub/migrate-plan-app-index/v1",
        "app_key": planning.app_key,
        "latest_run_id": run_id,
        "latest_run_path": paths.run_dir.display().to_string(),
        "run_state": run_state,
        "repo_fingerprint": planning.repo_fingerprint,
        "updated_at": now
    });

    write_text_atomically(&paths.spec_markdown, &spec_markdown)?;
    write_json_atomically(&paths.spec_meta, &spec_meta)?;
    write_json_atomically(&paths.run_json, &run_json)?;
    write_json_atomically(&paths.approval_json, &approval_json)?;
    write_json_atomically(&paths.latest_run_json, &latest_run_json)?;

    append_receipt(
        &paths.receipts_jsonl,
        &now,
        "run_created",
        None,
        None,
        &paths.run_json,
        &sha256_hex(&serde_json::to_string(&run_json)?),
        "migrate planning run scaffold created",
    )?;
    append_receipt(
        &paths.receipts_jsonl,
        &now,
        "spec_draft_written",
        None,
        None,
        &paths.spec_markdown,
        &spec_sha,
        "migrate planning spec draft written",
    )?;

    let mut next_required_stages = vec![];
    if planning.mode == PlanningMode::FullConsensus {
        let discover_markdown =
            render_discover_stage_markdown(root, candidate, env_refs, selected_sdk);
        let discover_sha = sha256_hex(&discover_markdown);
        let discover_meta = json!({
            "schema_version": "axhub/migrate-plan-stage/v1",
            "run_id": run_id,
            "app_key": planning.app_key,
            "stage": "discover",
            "stage_n": 1,
            "state": "complete",
            "artifact_sha256": discover_sha,
            "created_at": now,
            "updated_at": now
        });
        write_text_atomically(&paths.discover_markdown, &discover_markdown)?;
        write_json_atomically(&paths.discover_meta, &discover_meta)?;
        append_receipt(
            &paths.receipts_jsonl,
            &now,
            "stage_written",
            Some("discover"),
            Some(1),
            &paths.discover_markdown,
            &discover_sha,
            "discover stage scaffold written",
        )?;
        next_required_stages = vec!["planner", "architect", "critic", "reviewer"]
            .into_iter()
            .map(str::to_string)
            .collect();
    } else {
        append_receipt(
            &paths.receipts_jsonl,
            &now,
            "approval_requested",
            None,
            None,
            &paths.approval_json,
            &sha256_hex(&serde_json::to_string(&approval_json)?),
            "spec-only migrate planning requested approval",
        )?;
    }

    let mut out_paths = BTreeMap::new();
    out_paths.insert(
        "spec_markdown".to_string(),
        paths.spec_markdown.display().to_string(),
    );
    out_paths.insert(
        "spec_meta".to_string(),
        paths.spec_meta.display().to_string(),
    );
    out_paths.insert("run_json".to_string(), paths.run_json.display().to_string());
    out_paths.insert(
        "approval_json".to_string(),
        paths.approval_json.display().to_string(),
    );
    out_paths.insert(
        "receipts_jsonl".to_string(),
        paths.receipts_jsonl.display().to_string(),
    );
    out_paths.insert(
        "latest_run_json".to_string(),
        paths.latest_run_json.display().to_string(),
    );
    if planning.mode == PlanningMode::FullConsensus {
        out_paths.insert(
            "discover_markdown".to_string(),
            paths.discover_markdown.display().to_string(),
        );
        out_paths.insert(
            "discover_meta".to_string(),
            paths.discover_meta.display().to_string(),
        );
    }

    Ok(Some(PlanningPersistenceOutput {
        schema_version: "migrate-plan-persistence/v1".to_string(),
        persisted: true,
        mode: planning.mode,
        reason: if planning.mode == PlanningMode::SpecOnly {
            "spec_only_pending_approval_written".to_string()
        } else {
            "full_consensus_scaffold_written".to_string()
        },
        run_id: Some(run_id),
        spec_id: Some(spec_id),
        run_state: Some(run_state.to_string()),
        approval_state: Some(approval_state.to_string()),
        wrote_latest_pointer: false,
        next_required_stages,
        paths: out_paths,
    }))
}

struct PersistencePaths {
    run_dir: PathBuf,
    spec_markdown: PathBuf,
    spec_meta: PathBuf,
    run_json: PathBuf,
    approval_json: PathBuf,
    receipts_jsonl: PathBuf,
    latest_run_json: PathBuf,
    discover_markdown: PathBuf,
    discover_meta: PathBuf,
}

fn persistence_paths(
    planning_root: &Path,
    app_key: &str,
    run_id: &str,
    spec_id: &str,
) -> PersistencePaths {
    let spec_app_dir = planning_root.join("spec").join("apps").join(app_key);
    let spec_dir = spec_app_dir.join("specs");
    let run_dir = planning_root.join("plan").join("runs").join(run_id);
    let stages_dir = run_dir.join("stages");
    PersistencePaths {
        run_dir: run_dir.clone(),
        spec_markdown: spec_dir.join(format!("{spec_id}.md")),
        spec_meta: spec_dir.join(format!("{spec_id}.meta.json")),
        run_json: run_dir.join("run.json"),
        approval_json: run_dir.join("approval.json"),
        receipts_jsonl: run_dir.join("receipts.jsonl"),
        latest_run_json: planning_root
            .join("plan")
            .join("apps")
            .join(app_key)
            .join("latest-run.json"),
        discover_markdown: stages_dir.join("01-discover.md"),
        discover_meta: stages_dir.join("01-discover.meta.json"),
    }
}

fn render_planning_spec_markdown(
    root: &Path,
    candidate: &AppCandidate,
    env_refs: &[EnvRef],
    suggested_manifest: &str,
    selected_sdk: Option<&SdkConversionCandidate>,
    planning: &PlanningPreview,
) -> String {
    let mut out = String::new();
    out.push_str("# AXHub migrate planning spec draft\n\n");
    out.push_str(&format!("- mode: {}\n", planning.mode.as_str()));
    out.push_str(&format!(
        "- escalation_reason: {}\n",
        planning.escalation_reason.as_str()
    ));
    out.push_str(&format!("- app_key: {}\n", planning.app_key));
    out.push_str(&format!("- app_path: {}\n", candidate.path));
    out.push_str(&format!("- repo_root: {}\n", root.display()));
    out.push_str(&format!(
        "- candidate_confidence: {:.2}\n\n",
        candidate.confidence
    ));
    out.push_str("## Candidate\n");
    out.push_str(&format!("- stack_hint: {}\n", candidate.stack_hint));
    out.push_str(&format!("- has_dockerfile: {}\n", candidate.has_dockerfile));
    out.push_str(&format!("- has_compose: {}\n", candidate.has_compose));
    if let Some(compose_file) = candidate.compose_file.as_deref() {
        out.push_str(&format!("- compose_file: {}\n", compose_file));
    }
    out.push_str("\n## Env refs\n");
    if env_refs.is_empty() {
        out.push_str("- none\n");
    } else {
        for env in env_refs {
            out.push_str(&format!("- {} ({})\n", env.name, env.scope));
        }
    }
    out.push_str("\n## Suggested manifest\n\n```yaml\n");
    out.push_str(suggested_manifest.trim_end());
    out.push_str("\n```\n");
    if let Some(selected_sdk) = selected_sdk {
        out.push_str("\n## SDK conversion\n");
        out.push_str(&format!("- language: {}\n", selected_sdk.language));
        out.push_str(&format!("- framework: {}\n", selected_sdk.framework));
        out.push_str(&format!(
            "- dependency_hint: {}\n",
            selected_sdk.dependency_hint
        ));
        out.push_str(&format!(
            "- wrapper_target_path: {}\n",
            selected_sdk.wrapper_target_path
        ));
        out.push_str(&format!("- risk: {}\n", selected_sdk.risk));
        if !selected_sdk.hard_stop_reasons.is_empty() {
            out.push_str("- hard_stop_reasons:\n");
            for reason in &selected_sdk.hard_stop_reasons {
                out.push_str(&format!("  - {}\n", reason));
            }
        }
    }
    out
}

fn render_discover_stage_markdown(
    root: &Path,
    candidate: &AppCandidate,
    env_refs: &[EnvRef],
    selected_sdk: Option<&SdkConversionCandidate>,
) -> String {
    let mut out = String::new();
    out.push_str("# Discover\n\n");
    out.push_str(&format!("- repo_root: {}\n", root.display()));
    out.push_str(&format!("- app_path: {}\n", candidate.path));
    out.push_str(&format!("- stack_hint: {}\n", candidate.stack_hint));
    out.push_str(&format!("- confidence: {:.2}\n", candidate.confidence));
    out.push_str("\n## Evidence\n");
    for env in env_refs {
        out.push_str(&format!("- env_ref: {} ({})\n", env.name, env.scope));
    }
    if let Some(selected_sdk) = selected_sdk {
        out.push_str(&format!("- language: {}\n", selected_sdk.language));
        out.push_str(&format!("- framework: {}\n", selected_sdk.framework));
        out.push_str(&format!(
            "- wrapper_target_path: {}\n",
            selected_sdk.wrapper_target_path
        ));
        for reason in &selected_sdk.hard_stop_reasons {
            out.push_str(&format!("- hard_stop_reason: {}\n", reason));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn append_receipt(
    path: &Path,
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
    atomic_jsonl::append_line(path, &serde_json::to_string(&line)?)?;
    Ok(())
}

fn ensure_repo_fingerprint_matches(path: &Path, expected: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(path)?;
    let parsed: Value = serde_json::from_str(&raw)?;
    if let Some(existing) = parsed.get("repo_fingerprint").and_then(Value::as_str) {
        if existing != expected {
            bail!("migrate-plan: repo fingerprint mismatch; workspace-shared planning root collision detected");
        }
    }
    Ok(())
}

fn write_text_atomically(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("txt")
    ));
    fs::write(&tmp, content)?;
    fs::rename(tmp, path)?;
    Ok(())
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

fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
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

#[cfg(test)]
mod tests {
    use super::{
        build_migrate_plan, build_migrate_plan_with_options, build_migrate_plan_with_selection,
        candidate_file_path, collect_files, path_to_portable_json, render_manifest,
        source_files_for_candidate, yaml_double_quote, AppCandidate, EnvRef, PlanningMode,
        ScanState, MAX_SCAN_BYTES, MAX_SCAN_DEPTH, MAX_SCAN_FILES, MAX_SOURCE_FILE_BYTES,
    };
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    #[test]
    fn path_to_portable_json_uses_forward_slashes() {
        assert_eq!(path_to_portable_json(Path::new(".")), ".");
        assert_eq!(path_to_portable_json(Path::new("apps\\web")), "apps/web");
    }

    #[test]
    fn source_files_for_candidate_sorts_paths_for_deterministic_preview_selection() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/z.ts"), "console.log('z')").unwrap();
        std::fs::write(temp.path().join("src/a.ts"), "console.log('a')").unwrap();
        let files = vec![PathBuf::from("src/z.ts"), PathBuf::from("src/a.ts")];

        let source_files = source_files_for_candidate(temp.path(), &files, Path::new("."));
        let ordered: Vec<String> = source_files
            .iter()
            .map(|file| path_to_portable_json(&file.rel_path))
            .collect();

        assert_eq!(ordered, vec!["src/a.ts", "src/z.ts"]);
    }

    #[test]
    fn build_migrate_plan_skips_large_source_files() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        let mut large = "console.log(process.env.HUGE_SECRET);\n"
            .as_bytes()
            .to_vec();
        large.resize(MAX_SOURCE_FILE_BYTES as usize + 1, b'a');
        std::fs::write(temp.path().join("src/main.ts"), large).unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();

        assert!(plan.candidates.iter().any(|c| c.stack_hint == "node"));
        assert!(!plan.env_refs.iter().any(|e| e.name == "HUGE_SECRET"));
    }

    #[test]
    fn build_migrate_plan_detects_cargo_stack_hint() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();

        assert!(plan
            .candidates
            .iter()
            .any(|c| c.path == "." && c.stack_hint == "rust"));
    }

    #[test]
    fn build_migrate_plan_marks_unsupported_sdk_languages_plan_only() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();
        let sdk_candidate = plan
            .sdk_conversion
            .candidates
            .iter()
            .find(|candidate| candidate.path == ".")
            .unwrap();

        assert_eq!(sdk_candidate.language, "rust");
        assert!(sdk_candidate
            .hard_stop_reasons
            .iter()
            .any(|reason| reason.contains("지원되는 SDK wrapper 언어 범위 밖")));
    }

    #[test]
    fn build_migrate_plan_emits_sdk_hints_for_supported_languages() {
        let temp = tempfile::tempdir().unwrap();

        let node = temp.path().join("apps/node");
        std::fs::create_dir_all(node.join("src")).unwrap();
        std::fs::write(
            node.join("package.json"),
            r#"{"dependencies":{"vite":"1.0.0"}}"#,
        )
        .unwrap();

        let python = temp.path().join("apps/python");
        std::fs::create_dir_all(python.join("src")).unwrap();
        std::fs::write(
            python.join("pyproject.toml"),
            "[project]\nname='demo'\ndependencies=['fastapi']\n",
        )
        .unwrap();

        let go = temp.path().join("apps/go");
        std::fs::create_dir_all(&go).unwrap();
        std::fs::write(go.join("go.mod"), "module example.com/demo\n").unwrap();

        let ruby = temp.path().join("apps/ruby");
        std::fs::create_dir_all(ruby.join("lib")).unwrap();
        std::fs::write(
            ruby.join("Gemfile"),
            "source 'https://rubygems.org'\ngem 'sinatra'\n",
        )
        .unwrap();

        let java = temp.path().join("apps/java");
        std::fs::create_dir_all(java.join("src/main/java")).unwrap();
        std::fs::write(java.join("build.gradle"), "plugins { id 'java-library' }\n").unwrap();

        let kotlin = temp.path().join("apps/kotlin");
        std::fs::create_dir_all(kotlin.join("src/main/kotlin")).unwrap();
        std::fs::write(
            kotlin.join("build.gradle.kts"),
            "plugins { kotlin(\"jvm\") version \"2.4.0\" }\n",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();
        let sdk = &plan.sdk_conversion.candidates;

        let node_candidate = sdk
            .iter()
            .find(|candidate| candidate.path == "apps/node")
            .unwrap();
        assert_eq!(node_candidate.framework, "vite");
        assert_eq!(node_candidate.dependency_hint, "npm install @ax-hub/sdk");
        assert_eq!(node_candidate.wrapper_target_path, "apps/node/src/axhub.ts");
        assert!(node_candidate
            .wrapper_preview
            .contains("import { AxHubClient } from '@ax-hub/sdk'"));
        assert!(node_candidate
            .wrapper_preview
            .contains("process.env.AX_HUB_PAT"));

        let python_candidate = sdk
            .iter()
            .find(|candidate| candidate.path == "apps/python")
            .unwrap();
        assert_eq!(python_candidate.framework, "fastapi");
        assert_eq!(python_candidate.dependency_hint, "pip install axhub-sdk");
        assert_eq!(
            python_candidate.wrapper_target_path,
            "apps/python/src/axhub_client.py"
        );
        assert!(python_candidate
            .wrapper_preview
            .contains("from axhub_sdk import AxHubClient, TokenType"));
        assert!(python_candidate.wrapper_preview.contains("AXHUB_TOKEN"));

        let go_candidate = sdk
            .iter()
            .find(|candidate| candidate.path == "apps/go")
            .unwrap();
        assert_eq!(
            go_candidate.dependency_hint,
            "go get github.com/jocoding-ax-partners/axhub-sdk-go"
        );
        assert_eq!(go_candidate.wrapper_target_path, "apps/go/axhub_client.go");
        assert!(go_candidate.wrapper_preview.contains("package main"));
        assert!(go_candidate.wrapper_preview.contains("NewAxHubClient"));

        let ruby_candidate = sdk
            .iter()
            .find(|candidate| candidate.path == "apps/ruby")
            .unwrap();
        assert_eq!(ruby_candidate.framework, "sinatra");
        assert_eq!(ruby_candidate.dependency_hint, "bundle add axhub-sdk");
        assert_eq!(
            ruby_candidate.wrapper_target_path,
            "apps/ruby/lib/axhub_client.rb"
        );
        assert!(ruby_candidate
            .wrapper_preview
            .contains("require 'axhub_sdk'"));
        assert!(ruby_candidate.wrapper_preview.contains("AXHUB_CLIENT"));

        let java_candidate = sdk
            .iter()
            .find(|candidate| candidate.path == "apps/java")
            .unwrap();
        assert_eq!(
            java_candidate.dependency_hint,
            "implementation(\"ai.axhub:axhub-sdk-java:<version>\")"
        );
        assert_eq!(
            java_candidate.wrapper_target_path,
            "apps/java/src/main/java/ai/axhub/sdk/AxhubClientFactory.java"
        );
        assert!(java_candidate
            .wrapper_preview
            .contains("package ai.axhub.sdk;"));
        assert!(java_candidate
            .wrapper_preview
            .contains("AxhubClientFactory"));

        let kotlin_candidate = sdk
            .iter()
            .find(|candidate| candidate.path == "apps/kotlin")
            .unwrap();
        assert_eq!(
            kotlin_candidate.dependency_hint,
            "implementation(\"ai.axhub:axhub-sdk-kotlin:<version>\")"
        );
        assert_eq!(
            kotlin_candidate.wrapper_target_path,
            "apps/kotlin/src/main/kotlin/ai/axhub/sdk/AxhubClientFactory.kt"
        );
        assert!(kotlin_candidate
            .wrapper_preview
            .contains("package ai.axhub.sdk"));
        assert!(kotlin_candidate
            .wrapper_preview
            .contains("fun buildAxhubClient()"));
    }

    #[test]
    fn build_migrate_plan_detects_docker_compose_yaml() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("docker-compose.yaml"),
            "services:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();

        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].path, ".");
        assert!(plan.candidates[0].has_compose);
        assert_eq!(
            plan.candidates[0].compose_file.as_deref(),
            Some("docker-compose.yaml")
        );
        assert!(plan.container_contracts.compose);
    }

    #[test]
    fn build_migrate_plan_scopes_candidate_env_refs() {
        let temp = tempfile::tempdir().unwrap();
        let web = temp.path().join("apps/web");
        std::fs::create_dir_all(web.join("src")).unwrap();
        std::fs::write(web.join("package.json"), "{}").unwrap();
        std::fs::write(
            web.join("src/main.ts"),
            "console.log(process.env.WEB_SECRET);",
        )
        .unwrap();
        let api = temp.path().join("services/api");
        std::fs::create_dir_all(api.join("src")).unwrap();
        std::fs::write(api.join("package.json"), "{}").unwrap();
        std::fs::write(
            api.join("src/main.ts"),
            "console.log(process.env.API_SECRET);",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();
        let web_candidate = plan
            .candidates
            .iter()
            .find(|candidate| candidate.path == "apps/web")
            .unwrap();
        let api_candidate = plan
            .candidates
            .iter()
            .find(|candidate| candidate.path == "services/api")
            .unwrap();

        assert_eq!(web_candidate.env_refs, vec!["WEB_SECRET"]);
        assert_eq!(api_candidate.env_refs, vec!["API_SECRET"]);
        assert!(plan.env_refs.iter().any(|env| env.name == "WEB_SECRET"));
        assert!(plan.env_refs.iter().any(|env| env.name == "API_SECRET"));
        assert!(plan.suggested_manifest.contains("WEB_SECRET"));
        assert!(!plan.suggested_manifest.contains("API_SECRET"));
    }

    #[test]
    fn build_migrate_plan_detects_sdk_data_auth_candidates_and_hard_stops() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("package.json"),
            r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(
            temp.path().join("src/db.ts"),
            "export async function rows(db: any) { return db.query('select * from users'); }",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("src/auth.ts"),
            "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();
        let sdk_candidate = plan
            .sdk_conversion
            .candidates
            .iter()
            .find(|candidate| candidate.path == ".")
            .unwrap();

        assert_eq!(sdk_candidate.language, "node");
        assert_eq!(sdk_candidate.wrapper_target_path, "src/axhub.ts");
        assert!(sdk_candidate
            .data_candidates
            .iter()
            .any(|candidate| candidate.file == "src/db.ts"));
        assert!(sdk_candidate
            .auth_candidates
            .iter()
            .any(|candidate| candidate.file == "src/auth.ts"));
        assert_eq!(sdk_candidate.risk, "high");
        assert!(sdk_candidate
            .hard_stop_reasons
            .iter()
            .any(|reason| reason.contains("raw query 중심 데이터 접근")));
        assert!(sdk_candidate
            .hard_stop_reasons
            .iter()
            .any(|reason| reason.contains("검증 명령 또는 테스트 anchor")));
        assert!(sdk_candidate
            .expected_diff_summary
            .iter()
            .any(|line| line.contains("plan-only review for 1 data candidate")));
        assert!(sdk_candidate
            .expected_diff_summary
            .iter()
            .any(|line| line.contains("preview only")));
    }

    #[test]
    fn build_migrate_plan_marks_nested_core_entries_preview_only() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("package.json"),
            r#"{"dependencies":{"vite":"1.0.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::create_dir_all(temp.path().join("tests")).unwrap();
        std::fs::write(
            temp.path().join("src/main.ts"),
            "export const boot = () => console.log('boot');",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("src/client.ts"),
            "export const client = () => 'ok';",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("tests/app.test.ts"),
            "expect(true).toBe(true);",
        )
        .unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();
        let sdk_candidate = plan
            .sdk_conversion
            .candidates
            .iter()
            .find(|candidate| candidate.path == ".")
            .unwrap();

        assert!(sdk_candidate
            .hard_stop_reasons
            .iter()
            .any(|reason| reason.contains("wide diff 또는 핵심 진입 파일 blast radius")));
    }

    #[test]
    fn build_migrate_plan_renders_selected_non_first_candidate_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let api = temp.path().join("apps/api");
        std::fs::create_dir_all(api.join("src")).unwrap();
        std::fs::write(api.join("Dockerfile"), "FROM scratch\n").unwrap();
        std::fs::write(api.join("src/main.ts"), "process.env.API_SECRET;").unwrap();
        let web = temp.path().join("apps/web");
        std::fs::create_dir_all(web.join("src")).unwrap();
        std::fs::write(
            web.join("docker-compose.yaml"),
            "services:\n  web:\n    image: nginx\n",
        )
        .unwrap();
        std::fs::write(web.join("src/main.ts"), "process.env.WEB_SECRET;").unwrap();

        let plan = build_migrate_plan_with_selection(temp.path(), Some("apps/web")).unwrap();

        assert_eq!(plan.candidates[0].path, "apps/api");
        assert!(plan
            .suggested_manifest
            .contains("compose_file: \"apps/web/docker-compose.yaml\""));
        assert!(plan.suggested_manifest.contains("WEB_SECRET"));
        assert!(!plan.suggested_manifest.contains("API_SECRET"));
        assert!(!plan.suggested_manifest.contains("apps/api/Dockerfile"));
    }

    #[test]
    fn build_migrate_plan_rejects_selected_root_escape_path() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();

        let error = build_migrate_plan_with_selection(temp.path(), Some("../outside")).unwrap_err();

        assert!(error
            .to_string()
            .contains("선택한 앱 경로가 안전하지 않아요"));
    }

    #[test]
    fn build_migrate_plan_respects_max_scan_depth() {
        let temp = tempfile::tempdir().unwrap();
        let mut deep = temp.path().to_path_buf();
        for index in 0..=MAX_SCAN_DEPTH {
            deep = deep.join(format!("level-{index}"));
        }
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("package.json"), "{}").unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();

        assert!(plan.candidates.is_empty());
    }

    #[test]
    fn collect_files_respects_file_and_total_caps() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..(MAX_SCAN_FILES + 10) {
            std::fs::write(temp.path().join(format!("file-{index}.txt")), "x").unwrap();
        }
        let mut scan = ScanState::default();
        collect_files(temp.path(), temp.path(), 0, &mut scan).unwrap();
        assert!(scan.files.len() <= MAX_SCAN_FILES);

        let large_dir = tempfile::tempdir().unwrap();
        for index in 0..60 {
            let file =
                std::fs::File::create(large_dir.path().join(format!("blob-{index}.bin"))).unwrap();
            file.set_len(MAX_SOURCE_FILE_BYTES - 1).unwrap();
        }
        let mut scan = ScanState::default();
        collect_files(large_dir.path(), large_dir.path(), 0, &mut scan).unwrap();
        assert!(scan.total_bytes <= MAX_SCAN_BYTES);
        assert!(scan.files.len() < 60);
    }

    #[cfg(unix)]
    #[test]
    fn build_migrate_plan_skips_symlink_root_escape() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("package.json"), "{}").unwrap();
        std::fs::write(
            outside.path().join("index.ts"),
            "console.log(process.env.OUTSIDE_SECRET);",
        )
        .unwrap();
        symlink(outside.path(), root.path().join("outside-link")).unwrap();

        let plan = build_migrate_plan(root.path()).unwrap();

        assert!(plan.candidates.is_empty());
        assert!(!plan.env_refs.iter().any(|e| e.name == "OUTSIDE_SECRET"));
    }

    #[cfg(unix)]
    #[test]
    fn build_migrate_plan_skips_symlink_loop() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();
        symlink(temp.path(), temp.path().join("loop")).unwrap();

        let plan = build_migrate_plan(temp.path()).unwrap();

        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].stack_hint, "node");
    }

    #[test]
    fn yaml_double_quote_escapes_structural_characters() {
        assert_eq!(yaml_double_quote("name: value"), "\"name: value\"");
        assert_eq!(yaml_double_quote("line\nbreak"), "\"line\\nbreak\"");
        assert_eq!(
            yaml_double_quote("quote\"slash\\"),
            "\"quote\\\"slash\\\\\""
        );
    }

    #[test]
    fn build_migrate_plan_persists_spec_only_pending_approval_for_selected_monorepo_candidate() {
        let temp = tempfile::tempdir().unwrap();
        let web = temp.path().join("apps/web");
        let api = temp.path().join("services/api");
        std::fs::create_dir_all(web.join("src")).unwrap();
        std::fs::create_dir_all(web.join("tests")).unwrap();
        std::fs::create_dir_all(&api).unwrap();
        std::fs::write(
            web.join("package.json"),
            r#"{"dependencies":{"vite":"1.0.0"}}"#,
        )
        .unwrap();
        std::fs::write(web.join("src/web.ts"), "export const web = true;").unwrap();
        std::fs::write(web.join("tests/app.test.ts"), "expect(true).toBe(true);").unwrap();
        std::fs::write(api.join("go.mod"), "module example.com/api\n").unwrap();

        let plan = build_migrate_plan_with_options(temp.path(), Some("apps/web"), true).unwrap();
        let persistence = plan.planning_persistence.as_ref().unwrap();
        let app_key = plan.planning.as_ref().unwrap().app_key.clone();
        assert_eq!(persistence.mode, PlanningMode::SpecOnly);
        assert_eq!(persistence.reason, "spec_only_pending_approval_written");
        assert_eq!(persistence.run_state.as_deref(), Some("pending_approval"));
        assert_eq!(
            persistence.approval_state.as_deref(),
            Some("pending_approval")
        );
        assert!(persistence.next_required_stages.is_empty());

        let run_json: Value = serde_json::from_str(
            &std::fs::read_to_string(persistence.paths.get("run_json").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(run_json["mode"], "spec_only");
        assert_eq!(run_json["state"], "pending_approval");
        assert_eq!(run_json["stage_order"], Value::Array(vec![]));

        let approval_json: Value = serde_json::from_str(
            &std::fs::read_to_string(persistence.paths.get("approval_json").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(approval_json["state"], "pending_approval");
        assert_eq!(approval_json["required_before_execution"], true);

        let receipts =
            std::fs::read_to_string(persistence.paths.get("receipts_jsonl").unwrap()).unwrap();
        assert!(receipts.contains("run_created"));
        assert!(receipts.contains("spec_draft_written"));
        assert!(receipts.contains("approval_requested"));
        assert!(!temp
            .path()
            .join(".axhub/spec/apps")
            .join(app_key)
            .join("latest.json")
            .exists());
    }

    #[test]
    fn build_migrate_plan_persists_full_consensus_scaffold_for_hard_stop_candidate() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(
            temp.path().join("package.json"),
            r#"{"dependencies":{"next-auth":"1.0.0"}}"#,
        )
        .unwrap();
        std::fs::write(
            temp.path().join("src/db.ts"),
            "export async function rows(db: any) { return db.query('select * from users'); }",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("src/auth.ts"),
            "import passport from 'passport';\nexport const current_user = (req: any) => req.user;",
        )
        .unwrap();

        let plan = build_migrate_plan_with_options(temp.path(), Some("."), true).unwrap();
        let persistence = plan.planning_persistence.as_ref().unwrap();
        let app_key = plan.planning.as_ref().unwrap().app_key.clone();
        assert_eq!(persistence.mode, PlanningMode::FullConsensus);
        assert_eq!(persistence.reason, "full_consensus_scaffold_written");
        assert_eq!(persistence.run_state.as_deref(), Some("running"));
        assert_eq!(
            persistence.approval_state.as_deref(),
            Some("needs_revision")
        );
        assert_eq!(
            persistence.next_required_stages,
            vec!["planner", "architect", "critic", "reviewer"]
        );

        let run_json: Value = serde_json::from_str(
            &std::fs::read_to_string(persistence.paths.get("run_json").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(run_json["mode"], "full_consensus");
        assert_eq!(run_json["state"], "running");
        assert_eq!(run_json["stage_order"].as_array().unwrap().len(), 5);

        assert!(std::path::Path::new(persistence.paths.get("discover_markdown").unwrap()).exists());
        assert!(std::path::Path::new(persistence.paths.get("discover_meta").unwrap()).exists());
        assert!(!temp
            .path()
            .join(".axhub/spec/apps")
            .join(app_key)
            .join("latest.json")
            .exists());
        let receipts =
            std::fs::read_to_string(persistence.paths.get("receipts_jsonl").unwrap()).unwrap();
        assert!(receipts.contains("run_created"));
        assert!(receipts.contains("spec_draft_written"));
        assert!(receipts.contains("stage_written"));
    }

    #[test]
    fn build_migrate_plan_preserves_simple_flow_when_persist_flag_is_set() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::create_dir_all(temp.path().join("tests")).unwrap();
        std::fs::write(
            temp.path().join("package.json"),
            r#"{"dependencies":{"vite":"1.0.0"}}"#,
        )
        .unwrap();
        std::fs::write(temp.path().join("src/web.ts"), "export const web = true;").unwrap();
        std::fs::write(
            temp.path().join("tests/app.test.ts"),
            "expect(true).toBe(true);",
        )
        .unwrap();

        let plan = build_migrate_plan_with_options(temp.path(), Some("."), true).unwrap();
        let persistence = plan.planning_persistence.as_ref().unwrap();
        assert_eq!(persistence.mode, PlanningMode::Simple);
        assert_eq!(persistence.reason, "simple_flow_preserved");
        assert_eq!(persistence.persisted, false);
        assert!(persistence.paths.is_empty());
        assert!(!temp.path().join(".axhub/spec").exists());
        assert!(!temp.path().join(".axhub/plan").exists());
    }

    #[test]
    fn render_manifest_quotes_generated_scalars() {
        let candidate = AppCandidate {
            path: "weird:name".to_string(),
            stack_hint: "container".to_string(),
            has_dockerfile: false,
            has_compose: true,
            compose_file: Some("compose:\nfile.yml".to_string()),
            env_refs: vec![],
            confidence: 0.85,
        };
        let manifest = render_manifest(
            Some(&candidate),
            &[EnvRef {
                name: "DATABASE_URL".to_string(),
                scope: "runtime".to_string(),
            }],
        );

        assert!(manifest.contains("name: \"weird:name\""));
        assert!(manifest.contains("compose_file: \"weird:name/compose:\\nfile.yml\""));
        assert!(manifest.contains("name: \"DATABASE_URL\""));
    }

    #[test]
    fn render_manifest_prefixes_monorepo_container_paths() {
        let compose_candidate = AppCandidate {
            path: "apps/web".to_string(),
            stack_hint: "container".to_string(),
            has_dockerfile: false,
            has_compose: true,
            compose_file: Some("docker-compose.yaml".to_string()),
            env_refs: vec![],
            confidence: 0.85,
        };
        let manifest = render_manifest(Some(&compose_candidate), &[]);
        assert!(manifest.contains("compose_file: \"apps/web/docker-compose.yaml\""));

        let dockerfile_candidate = AppCandidate {
            path: "services/api".to_string(),
            stack_hint: "container".to_string(),
            has_dockerfile: true,
            has_compose: false,
            compose_file: None,
            env_refs: vec![],
            confidence: 0.95,
        };
        let manifest = render_manifest(Some(&dockerfile_candidate), &[]);
        assert!(manifest.contains("dockerfile: \"services/api/Dockerfile\""));
        assert_eq!(
            candidate_file_path("apps/web", "compose.yaml"),
            "apps/web/compose.yaml"
        );
    }

    #[test]
    fn absolute_hard_stop_reasons_are_never_overridable() {
        // SAFETY INVARIANT (devex-D1=C): a git rollback cannot un-leak a secret,
        // so these reasons must NEVER become overridable. If this flips, the
        // conversion could execute a credential-leaking patch behind a runtime
        // "강행" override instead of staying structurally plan-only.
        for code in ["secret_exposure", "custom_auth", "unsupported_language"] {
            assert!(
                !super::hard_stop_reason_overridable(code),
                "{code} must stay absolute (plan-only)"
            );
        }
    }

    #[test]
    fn power_user_hard_stop_reasons_are_overridable() {
        for code in ["missing_verification", "raw_query", "broad_diff"] {
            assert!(
                super::hard_stop_reason_overridable(code),
                "{code} should be overridable with the git checkpoint net"
            );
        }
    }

    #[test]
    fn unknown_hard_stop_reasons_fail_closed() {
        assert!(!super::hard_stop_reason_overridable("some_future_reason"));
    }

    #[test]
    fn render_wrapper_preview_carries_the_pack_constructor_contract() {
        // Cross-drift guard: each SDK knowledge pack §1 is a normalized copy of
        // render_wrapper_preview — the wrapper seed of truth. These per-language
        // tokens are the client-construction contract both must carry; if the
        // helper's wrapper drifts off them, the packs must be regenerated. The
        // pack side is asserted in tests/sdk-knowledge-pack.test.ts.
        let cases: &[(&str, &[&str])] = &[
            (
                "node",
                &[
                    "new AxHubClient(",
                    "tokenType: 'pat'",
                    "AX_HUB_PAT",
                    "defaultTenantId",
                ],
            ),
            (
                "python",
                &[
                    "AxHubClient(",
                    "token_type=TokenType.PAT",
                    "AXHUB_TOKEN",
                    "default_tenant_id",
                ],
            ),
            (
                "go",
                &[
                    "axhub.NewClient(",
                    "TokenTypePAT",
                    "AXHUB_TOKEN",
                    "DefaultTenantID",
                ],
            ),
            (
                "ruby",
                &[
                    "AxHub::Client.new(",
                    "token_type: :pat",
                    "AXHUB_TOKEN",
                    "default_tenant_id",
                ],
            ),
            (
                "java",
                &[
                    "AxHubClient.builder()",
                    ".tokenType(TokenType.PAT)",
                    "AXHUB_TOKEN",
                    ".defaultTenantId(",
                ],
            ),
            (
                "kotlin",
                &[
                    "AxHubKotlinClient(",
                    "tokenType = TokenType.PAT",
                    "AXHUB_TOKEN",
                    "defaultTenantId =",
                ],
            ),
        ];
        for (lang, tokens) in cases {
            let wrapper = super::render_wrapper_preview(lang, "", &[]);
            for token in *tokens {
                assert!(
                    wrapper.contains(token),
                    "{lang} wrapper seed missing contract token {token:?}"
                );
            }
        }
        // base_url is identical across the typed-init languages (node omits it).
        // Guards an api.axhub.ai change in the helper from drifting off the packs.
        for lang in ["python", "go", "ruby", "java", "kotlin"] {
            assert!(
                super::render_wrapper_preview(lang, "", &[]).contains("https://api.axhub.ai"),
                "{lang} wrapper seed missing base_url contract"
            );
        }
    }
}

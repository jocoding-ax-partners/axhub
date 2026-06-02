use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Serialize;

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

pub fn run_migrate_plan(args: &[String]) -> Result<i32> {
    let mut dir: Option<PathBuf> = None;
    let mut app_path: Option<String> = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                json = true;
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
    let output = build_migrate_plan_with_selection(&dir, app_path.as_deref())?;
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
    build_migrate_plan_with_selection(dir, None)
}

pub fn build_migrate_plan_with_selection(
    dir: &Path,
    selected_app_path: Option<&str>,
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
    Ok(MigratePlanOutput {
        schema_version: "migrate-plan/v1".to_string(),
        root: root.display().to_string(),
        source_dir: root.display().to_string(),
        monorepo: candidates.len() > 1,
        candidates,
        container_contracts,
        env_refs,
        suggested_manifest,
    })
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

#[cfg(test)]
mod tests {
    use super::{
        build_migrate_plan, build_migrate_plan_with_selection, candidate_file_path, collect_files,
        path_to_portable_json, render_manifest, yaml_double_quote, AppCandidate, EnvRef, ScanState,
        MAX_SCAN_BYTES, MAX_SCAN_DEPTH, MAX_SCAN_FILES, MAX_SOURCE_FILE_BYTES,
    };
    use std::path::Path;

    #[test]
    fn path_to_portable_json_uses_forward_slashes() {
        assert_eq!(path_to_portable_json(Path::new(".")), ".");
        assert_eq!(path_to_portable_json(Path::new("apps\\web")), "apps/web");
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
}

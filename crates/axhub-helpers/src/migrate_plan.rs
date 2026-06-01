use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Serialize;

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
            other => bail!("migrate-plan: 알 수 없는 옵션 {other}"),
        }
    }
    let dir = dir.unwrap_or(std::env::current_dir()?);
    let output = build_migrate_plan(&dir)?;
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
    let root =
        fs::canonicalize(dir).with_context(|| format!("{} 경로를 읽지 못했어요", dir.display()))?;
    if !root.is_dir() {
        bail!("{} 는 디렉터리가 아니에요", root.display());
    }
    let mut files = Vec::new();
    collect_files(&root, &root, 0, &mut files)?;
    let env_refs = scan_env_refs(&root, &files);
    let candidates = detect_candidates(&root, &files, &env_refs);
    let container_contracts = ContainerContracts {
        dockerfile: candidates.iter().any(|c| c.has_dockerfile),
        compose: candidates.iter().any(|c| c.has_compose),
    };
    let suggested_manifest = render_manifest(candidates.first(), &env_refs);
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

fn collect_files(root: &Path, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> Result<()> {
    if depth > 5 {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("{} 를 읽지 못했어요", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            ".git" | "node_modules" | "target" | "dist" | "build" | ".next" | "vendor"
        ) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, depth + 1, out)?;
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}

fn detect_candidates(root: &Path, files: &[PathBuf], env_refs: &[EnvRef]) -> Vec<AppCandidate> {
    let mut by_dir: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    for rel in files {
        let Some(file) = rel.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let dir = rel.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        if stack_for_marker(file).is_some()
            || matches!(
                file,
                "Dockerfile" | "docker-compose.yml" | "compose.yaml" | "compose.yml"
            )
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
        let compose_file = ["docker-compose.yml", "compose.yaml", "compose.yml"]
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
    if out.is_empty() && root.join("Dockerfile").exists() {
        out.push(AppCandidate {
            path: ".".to_string(),
            stack_hint: "container".to_string(),
            has_dockerfile: true,
            has_compose: false,
            compose_file: None,
            env_refs: env_refs.iter().map(|e| e.name.clone()).collect(),
            confidence: 0.95,
        });
    }
    out
}

fn stack_for_marker(file: &str) -> Option<&'static str> {
    match file {
        "package.json" => Some("node"),
        "requirements.txt" | "pyproject.toml" => Some("python"),
        "go.mod" => Some("go"),
        "Gemfile" => Some("ruby"),
        "pom.xml" | "build.gradle" => Some("java"),
        "build.gradle.kts" | "settings.gradle.kts" => Some("kotlin"),
        _ => None,
    }
}

fn scan_env_refs(root: &Path, files: &[PathBuf]) -> Vec<EnvRef> {
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
        let Ok(body) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for re in &patterns {
            for cap in re.captures_iter(&body) {
                let name = cap[1].to_string();
                refs.insert(EnvRef {
                    scope: scope_for_env(&name).to_string(),
                    name,
                });
            }
        }
    }
    refs.into_iter().collect()
}

fn env_refs_for_candidate(dir: &Path, env_refs: &[EnvRef]) -> Vec<String> {
    if dir.as_os_str().is_empty() || dir == Path::new(".") {
        return env_refs.iter().map(|e| e.name.clone()).collect();
    }
    // The light pre-scan intentionally keeps env extraction simple. Candidate
    // records expose the global env-ref names so the skill can show one compact
    // confirmation card; backend detection remains authoritative.
    env_refs.iter().map(|e| e.name.clone()).collect()
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
    let mut out = format!("version: axhub/v1\nname: {name}\nbuild:\n  strategy: auto\n");
    if let Some(c) = candidate {
        if let Some(compose) = &c.compose_file {
            out.push_str(&format!(
                "  deploy_method: compose\n  compose_file: {compose}\n"
            ));
        }
    }
    if !env_refs.is_empty() {
        out.push_str("env:\n  required:\n");
        for env in env_refs {
            out.push_str(&format!(
                "    - {{ name: {}, scope: {} }}\n",
                env.name, env.scope
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::path_to_portable_json;
    use std::path::Path;

    #[test]
    fn path_to_portable_json_uses_forward_slashes() {
        assert_eq!(path_to_portable_json(Path::new(".")), ".");
        assert_eq!(path_to_portable_json(Path::new("apps\\web")), "apps/web");
    }
}

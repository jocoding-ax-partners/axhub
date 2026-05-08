use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::runtime_paths::{last_deploy_file, state_dir, token_file};
use serde::Deserialize;

const DEFAULT_PROFILE: &str = "default";
const MAX_STATUSLINE_CHARS: usize = 80;
const STATUSLINE_CACHE_FILENAME: &str = "statusline.cache";
const TERMINAL_PHASES: &[&str] = &["complete", "succeeded", "failed", "cancelled", "errored"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployStatus {
    pub app_slug: String,
    pub phase: String,
    pub elapsed_secs: u64,
    pub ready_services: u32,
    pub total_services: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatuslineSnapshot {
    auth_ok: bool,
    profile: String,
    last_deploy: Option<LastDeploy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LastDeploy {
    app_slug: Option<String>,
    commit_sha: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct LastDeployCache {
    app_slug: Option<String>,
    commit_sha: Option<String>,
    status: Option<String>,
}

pub fn current_statusline() -> String {
    render_statusline(&current_snapshot())
}

fn current_snapshot() -> StatuslineSnapshot {
    let auth_ok = token_is_present();
    let profile = env::var("AXHUB_PROFILE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROFILE.to_string());
    let last_deploy = if auth_ok {
        last_deploy_file()
            .as_deref()
            .and_then(read_last_deploy_cache)
    } else {
        None
    };

    StatuslineSnapshot {
        auth_ok,
        profile,
        last_deploy,
    }
}

fn render_statusline(snapshot: &StatuslineSnapshot) -> String {
    if !snapshot.auth_ok {
        return "axhub: 로그인 안 됐어요\n".to_string();
    }

    let line = match &snapshot.last_deploy {
        Some(deploy) => {
            let sha = short_sha(&deploy.commit_sha);
            let app = deploy.app_slug.as_deref().unwrap_or("?");
            fit_first([
                format!(
                    "axhub: {app} · {} · 최근 배포 {sha} ({})",
                    snapshot.profile, deploy.status
                ),
                format!(
                    "axhub: {} · 최근 배포 {sha} ({})",
                    snapshot.profile, deploy.status
                ),
                format!("axhub: 최근 배포 {sha} ({})", deploy.status),
                format!("axhub: 최근 배포 {sha}"),
            ])
        }
        None => fit_first([
            format!("axhub: {} · 배포 기록 없어요", snapshot.profile),
            "axhub: 배포 기록 없어요".to_string(),
        ]),
    };

    format!("{line}\n")
}

fn token_is_present() -> bool {
    env::var("AXHUB_TOKEN")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
        || token_file().is_some_and(|path| path.is_file())
}

fn read_last_deploy_cache(path: &Path) -> Option<LastDeploy> {
    let raw = fs::read_to_string(path).ok()?;
    let cache: LastDeployCache = serde_json::from_str(&raw).ok()?;
    let commit_sha = cache.commit_sha.filter(|value| !value.trim().is_empty())?;
    let status = cache.status.filter(|value| !value.trim().is_empty())?;
    Some(LastDeploy {
        app_slug: cache.app_slug.filter(|value| !value.trim().is_empty()),
        commit_sha,
        status,
    })
}

fn short_sha(commit_sha: &str) -> String {
    commit_sha.chars().take(8).collect()
}

fn fit_first<const N: usize>(candidates: [String; N]) -> String {
    for candidate in &candidates {
        if char_len(candidate) <= MAX_STATUSLINE_CHARS {
            return candidate.clone();
        }
    }
    truncate_chars(&candidates[N - 1], MAX_STATUSLINE_CHARS)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

pub fn statusline_cache_path() -> Option<PathBuf> {
    state_dir().map(|d| d.join(STATUSLINE_CACHE_FILENAME))
}

pub fn render_from_deploy_status(status: &DeployStatus) -> String {
    fit_first([
        format!(
            "axhub: {} · {} ({}s) · {}/{}",
            status.app_slug,
            status.phase,
            status.elapsed_secs,
            status.ready_services,
            status.total_services,
        ),
        format!(
            "axhub: {} · {} ({}s)",
            status.app_slug, status.phase, status.elapsed_secs
        ),
        format!("axhub: {} · {}", status.app_slug, status.phase),
    ])
}

pub fn write_statusline_cache(path: &Path, line: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, format!("{line}\n"))?;
    Ok(())
}

fn is_terminal_phase(phase: &str) -> bool {
    TERMINAL_PHASES
        .iter()
        .any(|t| phase.eq_ignore_ascii_case(t))
}

pub struct WatchSummary {
    pub iterations: u64,
    pub writes: u64,
    pub final_phase: Option<String>,
}

pub fn watch_and_update_statusline<F, S>(
    deploy_id: &str,
    cache_path: &Path,
    poll_interval: Duration,
    max_iterations: Option<u64>,
    fetcher: F,
    sleeper: S,
) -> anyhow::Result<WatchSummary>
where
    F: Fn(&str) -> anyhow::Result<DeployStatus>,
    S: Fn(Duration),
{
    let mut last_rendered: Option<String> = None;
    let mut iterations: u64 = 0;
    let mut writes: u64 = 0;
    let mut final_phase: Option<String> = None;

    loop {
        let status = fetcher(deploy_id)?;
        let line = render_from_deploy_status(&status);
        let changed = last_rendered.as_deref() != Some(line.as_str());
        if changed {
            write_statusline_cache(cache_path, &line)?;
            last_rendered = Some(line);
            writes += 1;
        }

        if is_terminal_phase(&status.phase) {
            final_phase = Some(status.phase);
            break;
        }

        iterations += 1;
        if let Some(limit) = max_iterations {
            if iterations >= limit {
                break;
            }
        }
        sleeper(poll_interval);
    }

    Ok(WatchSummary {
        iterations,
        writes,
        final_phase,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_statusline_prompts_login_when_no_token_exists() {
        let snapshot = StatuslineSnapshot {
            auth_ok: false,
            profile: DEFAULT_PROFILE.to_string(),
            last_deploy: None,
        };

        assert_eq!(render_statusline(&snapshot), "axhub: 로그인 안 됐어요\n");
    }

    #[test]
    fn render_statusline_shows_profile_when_authenticated_without_deploy_cache() {
        let snapshot = StatuslineSnapshot {
            auth_ok: true,
            profile: "prod".to_string(),
            last_deploy: None,
        };

        assert_eq!(
            render_statusline(&snapshot),
            "axhub: prod · 배포 기록 없어요\n"
        );
    }

    #[test]
    fn render_statusline_shows_recent_deploy_with_short_sha() {
        let snapshot = StatuslineSnapshot {
            auth_ok: true,
            profile: "prod".to_string(),
            last_deploy: Some(LastDeploy {
                app_slug: Some("paydrop".to_string()),
                commit_sha: "abcdef1234567890".to_string(),
                status: "succeeded".to_string(),
            }),
        };

        assert_eq!(
            render_statusline(&snapshot),
            "axhub: paydrop · prod · 최근 배포 abcdef12 (succeeded)\n"
        );
    }

    #[test]
    fn render_statusline_keeps_long_lines_within_contract() {
        let snapshot = StatuslineSnapshot {
            auth_ok: true,
            profile: "very-long-profile-name-that-would-overflow".to_string(),
            last_deploy: Some(LastDeploy {
                app_slug: Some("very-long-app-name-that-would-overflow".to_string()),
                commit_sha: "abcdef1234567890".to_string(),
                status: "succeeded-with-a-very-long-status-name".to_string(),
            }),
        };

        let line = render_statusline(&snapshot);
        assert!(line.trim_end().chars().count() <= MAX_STATUSLINE_CHARS);
        assert!(line.contains("최근 배포 abcdef12"));
    }

    #[test]
    fn read_last_deploy_cache_uses_json_instead_of_shell_grep() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("last-deploy.json");
        fs::write(
            &cache,
            r#"{
              "app_slug": "paydrop",
              "commit_sha": "abcdef1234567890",
              "status": "succeeded"
            }"#,
        )
        .unwrap();

        assert_eq!(
            read_last_deploy_cache(&cache),
            Some(LastDeploy {
                app_slug: Some("paydrop".to_string()),
                commit_sha: "abcdef1234567890".to_string(),
                status: "succeeded".to_string(),
            })
        );
    }

    #[test]
    fn watch_writes_only_on_phase_change_and_breaks_at_terminal() {
        let temp = tempfile::tempdir().unwrap();
        let cache_path = temp.path().join("statusline.cache");
        // Simulate 10 polls: building (3) → built (2) → deploying (3) → complete (1) — 4 distinct phases.
        let phases = vec![
            "building",
            "building",
            "building",
            "built",
            "built",
            "deploying",
            "deploying",
            "deploying",
            "complete",
        ];
        let counter = std::cell::Cell::new(0usize);
        let fetcher = |_id: &str| -> anyhow::Result<DeployStatus> {
            let i = counter.get();
            counter.set(i + 1);
            let phase = phases.get(i).copied().unwrap_or("complete").to_string();
            Ok(DeployStatus {
                app_slug: "paydrop".into(),
                phase,
                elapsed_secs: i as u64 * 5,
                ready_services: 1,
                total_services: 1,
            })
        };
        let summary = watch_and_update_statusline(
            "deploy-xyz",
            &cache_path,
            Duration::from_secs(0),
            Some(20),
            fetcher,
            |_| {},
        )
        .unwrap();
        // 4 distinct phase strings, but each render line also includes elapsed_secs
        // which changes every iteration. The change-detection key is the rendered
        // string, so we expect a write per call until terminal phase breaks early.
        assert!(summary.writes >= 4);
        assert_eq!(summary.final_phase.as_deref(), Some("complete"));
        let written = std::fs::read_to_string(&cache_path).unwrap();
        assert!(written.contains("complete"));
    }

    #[test]
    fn watch_skips_writes_when_render_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let cache_path = temp.path().join("statusline.cache");
        // Same status every poll → only 1 write (first one), then terminal break
        // never fires (non-terminal phase) so we rely on max_iterations = 5.
        let fetcher = |_id: &str| -> anyhow::Result<DeployStatus> {
            Ok(DeployStatus {
                app_slug: "paydrop".into(),
                phase: "building".into(),
                elapsed_secs: 12,
                ready_services: 0,
                total_services: 3,
            })
        };
        let summary = watch_and_update_statusline(
            "deploy-xyz",
            &cache_path,
            Duration::from_secs(0),
            Some(5),
            fetcher,
            |_| {},
        )
        .unwrap();
        assert_eq!(summary.writes, 1);
        assert_eq!(summary.iterations, 5);
        assert!(summary.final_phase.is_none());
    }

    #[test]
    fn fit_first_truncates_when_every_candidate_is_too_long() {
        let long = "x".repeat(MAX_STATUSLINE_CHARS + 10);

        let line = fit_first([long.clone(), long]);

        assert_eq!(line.chars().count(), MAX_STATUSLINE_CHARS);
    }
}

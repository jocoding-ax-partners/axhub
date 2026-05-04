use std::env;
use std::fs;
use std::path::Path;

use crate::runtime_paths::{last_deploy_file, token_file};
use serde::Deserialize;

const DEFAULT_PROFILE: &str = "default";
const MAX_STATUSLINE_CHARS: usize = 80;

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
}

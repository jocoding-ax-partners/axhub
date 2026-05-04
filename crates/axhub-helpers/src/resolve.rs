use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::preflight::{
    axhub_bin, default_runner, parse_auth_status, AuthStatus, SpawnResult, EXIT_AUTH, EXIT_OK,
    EXIT_USAGE,
};

pub const EXIT_NOT_FOUND: i32 = 67;
pub const DEFAULT_DEPLOY_ETA_SEC: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveArgs {
    pub intent: Option<String>,
    pub user_utterance: String,
}

pub fn parse_resolve_args(args: &[String]) -> ResolveArgs {
    let mut intent = None;
    let mut user_utterance = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--intent" if i + 1 < args.len() => {
                i += 1;
                intent = Some(args[i].clone());
            }
            "--user-utterance" if i + 1 < args.len() => {
                i += 1;
                user_utterance = args[i].clone();
            }
            _ => {}
        }
        i += 1;
    }
    ResolveArgs {
        intent,
        user_utterance,
    }
}

static STOP_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "배포",
        "배포해",
        "배포해줘",
        "올려",
        "올리자",
        "쏘자",
        "내보내자",
        "푸시한",
        "프로덕션에",
        "박아",
        "터트려",
        "공개해",
        "그거",
        "거",
        "좀",
        "해줘",
        "해",
        "하자",
        "해봐",
        "보여줘",
        "주세요",
        "을",
        "를",
        "에",
        "이",
        "가",
        "은",
        "는",
        "의",
        "도",
        "만",
        "지금",
        "방금",
        "어떻게",
        "됐어",
        "deploy",
        "ship",
        "release",
        "rollout",
        "launch",
        "push",
        "the",
        "to",
        "now",
        "please",
        "my",
        "app",
        "for",
        "of",
        "a",
        "an",
    ])
});
static SPLIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[\s,./!?;:()"'`\[\]{}]+"#).unwrap());
static SLUG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-]*$").unwrap());

pub fn extract_slug_candidate(utterance: &str) -> Option<String> {
    let normalized: String = utterance.nfkc().collect::<String>().to_lowercase();
    for tok in SPLIT_RE.split(&normalized).filter(|s| !s.is_empty()) {
        if STOP_WORDS.contains(tok) || tok.starts_with('-') || tok.len() < 2 {
            continue;
        }
        if SLUG_RE.is_match(tok) {
            return Some(tok.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppRecord {
    pub id: i64,
    pub slug: String,
    pub name: Option<String>,
}

pub fn parse_apps_list(stdout: &str) -> Option<Vec<AppRecord>> {
    let parsed: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let arr = parsed.as_array()?;
    Some(
        arr.iter()
            .filter_map(|item| {
                Some(AppRecord {
                    id: item.get("id")?.as_i64()?,
                    slug: item.get("slug")?.as_str()?.to_string(),
                    name: item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(ToOwned::to_owned),
                })
            })
            .collect(),
    )
}

pub fn filter_apps_by_slug(apps: &[AppRecord], candidate: &str) -> Vec<AppRecord> {
    let needle = candidate.nfkc().collect::<String>().to_lowercase();
    let prefix: Vec<_> = apps
        .iter()
        .filter(|a| a.slug.to_lowercase().starts_with(&needle))
        .cloned()
        .collect();
    if !prefix.is_empty() {
        return prefix;
    }
    apps.iter()
        .filter(|a| a.slug.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitContext {
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub git_repo: bool,
    pub git_has_commit: bool,
    pub git_init_needed: bool,
}

pub fn read_git_context<F>(runner: &F) -> GitContext
where
    F: Fn(&[&str]) -> SpawnResult,
{
    let safe = |cmd: &[&str]| runner(cmd);
    let repo = safe(&["git", "rev-parse", "--is-inside-work-tree"]);
    if repo.exit_code != EXIT_OK || repo.stdout.trim() != "true" {
        return GitContext {
            branch: None,
            commit_sha: None,
            commit_message: None,
            git_repo: false,
            git_has_commit: false,
            git_init_needed: true,
        };
    }

    let branch = safe(&["git", "branch", "--show-current"]);
    let sha = safe(&["git", "rev-parse", "HEAD"]);
    let msg = safe(&["git", "log", "-1", "--pretty=%s"]);
    let commit_sha = (sha.exit_code == EXIT_OK)
        .then(|| sha.stdout.trim().to_string())
        .filter(|s| !s.is_empty());
    let git_has_commit = commit_sha.is_some();
    GitContext {
        branch: (branch.exit_code == EXIT_OK)
            .then(|| branch.stdout.trim().to_string())
            .filter(|s| !s.is_empty()),
        commit_sha,
        commit_message: (msg.exit_code == EXIT_OK)
            .then(|| msg.stdout.trim().to_string())
            .filter(|s| !s.is_empty()),
        git_repo: true,
        git_has_commit,
        git_init_needed: false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolveOutput {
    pub profile: Option<String>,
    pub endpoint: Option<String>,
    pub app_id: Option<i64>,
    pub app_slug: Option<String>,
    pub candidate_slug: Option<String>,
    pub matched_apps: Vec<AppMatch>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub git_repo: bool,
    pub git_has_commit: bool,
    pub git_init_needed: bool,
    pub eta_sec: u64,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppMatch {
    pub id: i64,
    pub slug: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolveRun {
    pub output: ResolveOutput,
    pub exit_code: i32,
}

pub fn run_resolve(args: &[String]) -> ResolveRun {
    run_resolve_with_runner(args, default_runner)
}

pub fn run_resolve_with_runner<F>(args: &[String], runner: F) -> ResolveRun
where
    F: Fn(&[&str]) -> SpawnResult,
{
    let parsed_args = parse_resolve_args(args);
    let candidate = extract_slug_candidate(&parsed_args.user_utterance);
    let bin = axhub_bin();
    let base = ResolveOutput {
        profile: std::env::var("AXHUB_PROFILE")
            .ok()
            .filter(|s| !s.is_empty()),
        endpoint: std::env::var("AXHUB_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty()),
        app_id: None,
        app_slug: None,
        candidate_slug: candidate.clone(),
        matched_apps: vec![],
        branch: None,
        commit_sha: None,
        commit_message: None,
        git_repo: false,
        git_has_commit: false,
        git_init_needed: false,
        eta_sec: DEFAULT_DEPLOY_ETA_SEC,
        error: None,
    };
    let auth = parse_auth_status(&runner(&[&bin, "auth", "status", "--json"]).stdout);
    if !matches!(auth, AuthStatus::Ok { .. }) {
        let code = match auth {
            AuthStatus::Error { code, .. } => code,
            _ => unreachable!(),
        };
        return ResolveRun {
            output: ResolveOutput {
                error: Some(format!("auth_{code}")),
                ..base
            },
            exit_code: EXIT_AUTH,
        };
    }
    let apps = match parse_apps_list(&runner(&[&bin, "apps", "list", "--json"]).stdout) {
        Some(a) => a,
        None => {
            return ResolveRun {
                output: ResolveOutput {
                    error: Some("apps_list_parse_error".into()),
                    ..base
                },
                exit_code: EXIT_NOT_FOUND,
            }
        }
    };
    let matches = candidate
        .as_deref()
        .map(|c| filter_apps_by_slug(&apps, c))
        .unwrap_or_default();
    let git = read_git_context(&runner);
    let with_git = |mut out: ResolveOutput| {
        out.branch = git.branch.clone();
        out.commit_sha = git.commit_sha.clone();
        out.commit_message = git.commit_message.clone();
        out.git_repo = git.git_repo;
        out.git_has_commit = git.git_has_commit;
        out.git_init_needed = git.git_init_needed;
        out
    };
    if matches.is_empty() {
        return ResolveRun {
            output: with_git(ResolveOutput {
                error: Some(
                    if candidate.is_some() {
                        "app_not_found"
                    } else {
                        "no_candidate_slug"
                    }
                    .into(),
                ),
                ..base
            }),
            exit_code: EXIT_NOT_FOUND,
        };
    }
    if matches.len() > 1 {
        let matched_apps = matches
            .iter()
            .map(|a| AppMatch {
                id: a.id,
                slug: a.slug.clone(),
            })
            .collect();
        return ResolveRun {
            output: with_git(ResolveOutput {
                matched_apps,
                error: Some("app_ambiguous".into()),
                ..base
            }),
            exit_code: EXIT_USAGE,
        };
    }
    let sole = matches[0].clone();
    ResolveRun {
        output: with_git(ResolveOutput {
            app_id: Some(sole.id),
            app_slug: Some(sole.slug.clone()),
            matched_apps: vec![AppMatch {
                id: sole.id,
                slug: sole.slug,
            }],
            ..base
        }),
        exit_code: EXIT_OK,
    }
}

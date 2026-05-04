use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};

pub const MIN_AXHUB_CLI_VERSION: &str = "0.1.0";
pub const MAX_AXHUB_CLI_VERSION: &str = "0.11.0";
pub const EXIT_OK: i32 = 0;
pub const EXIT_USAGE: i32 = 64;
pub const EXIT_AUTH: i32 = 65;

static SEMVER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)\.(\d+)\.(\d+)").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn default_runner(cmd: &[&str]) -> SpawnResult {
    match crate::spawn::spawn_sync(cmd) {
        Ok(result) => SpawnResult {
            exit_code: result.exit_code.unwrap_or(1),
            stdout: result.stdout,
            stderr: result.stderr,
        },
        Err(e) => SpawnResult {
            exit_code: 127,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

pub fn axhub_bin() -> String {
    std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string())
}

pub fn extract_semver(text: &str) -> Option<String> {
    let caps = SEMVER_RE.captures(text)?;
    Some(format!("{}.{}.{}", &caps[1], &caps[2], &caps[3]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    Ok {
        user_email: String,
        user_id: i64,
        expires_at: String,
        scopes: Vec<String>,
    },
    Error {
        code: String,
        detail: String,
    },
}

impl AuthStatus {
    pub fn ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}

pub fn parse_auth_status(stdout: &str) -> AuthStatus {
    let parsed: serde_json::Value = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(_) => {
            return AuthStatus::Error {
                code: "parse_error".into(),
                detail: "auth status returned non-JSON".into(),
            }
        }
    };
    if let Some(code) = parsed.get("code").and_then(|v| v.as_str()) {
        return AuthStatus::Error {
            code: code.to_string(),
            detail: parsed
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };
    }
    if let Some(user_email) = parsed.get("user_email").and_then(|v| v.as_str()) {
        let scopes = parsed
            .get("scopes")
            .and_then(|v| v.as_array())
            .map(|xs| {
                xs.iter()
                    .filter_map(|x| x.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        return AuthStatus::Ok {
            user_email: user_email.to_string(),
            user_id: parsed.get("user_id").and_then(|v| v.as_i64()).unwrap_or(0),
            expires_at: parsed
                .get("expires_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            scopes,
        };
    }
    AuthStatus::Error {
        code: "unknown_shape".into(),
        detail: "auth status JSON missing expected fields".into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreflightOutput {
    pub cli_version: Option<String>,
    pub in_range: bool,
    pub cli_too_old: bool,
    pub cli_too_new: bool,
    pub cli_present: bool,
    pub auth_ok: bool,
    pub auth_error_code: Option<String>,
    pub scopes: Vec<String>,
    pub profile: Option<String>,
    pub endpoint: Option<String>,
    pub user_email: Option<String>,
    pub expires_at: Option<String>,
    pub current_app: Option<String>,
    pub current_env: Option<String>,
    pub last_deploy_id: Option<String>,
    pub last_deploy_status: Option<String>,
    pub plugin_version: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LastDeployCache {
    deployment_id: String,
    status: String,
    app_slug: Option<String>,
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn last_deploy_cache_path() -> PathBuf {
    home_dir().join(".cache/axhub-plugin/last-deploy.json")
}

fn read_last_deploy_cache() -> Option<LastDeployCache> {
    serde_json::from_str(&fs::read_to_string(last_deploy_cache_path()).ok()?).ok()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreflightRun {
    pub output: PreflightOutput,
    pub exit_code: i32,
}

pub fn run_preflight() -> PreflightRun {
    run_preflight_with_runner(default_runner)
}

pub fn run_preflight_with_runner<F>(runner: F) -> PreflightRun
where
    F: Fn(&[&str]) -> SpawnResult,
{
    let bin = axhub_bin();
    let version_result = runner(&[&bin, "--version"]);
    let cli_present = version_result.exit_code == EXIT_OK && !version_result.stdout.is_empty();
    let cli_version = if cli_present {
        extract_semver(&version_result.stdout)
    } else {
        None
    };
    let min = Version::parse(MIN_AXHUB_CLI_VERSION).unwrap();
    let max = Version::parse(MAX_AXHUB_CLI_VERSION).unwrap();
    let parsed = cli_version.as_deref().and_then(|s| Version::parse(s).ok());
    let in_range = parsed.as_ref().is_some_and(|v| v >= &min && v < &max);
    let too_old = parsed.as_ref().is_some_and(|v| v < &min);
    let too_new = parsed.as_ref().is_some_and(|v| v >= &max);

    let auth_status = if cli_present {
        parse_auth_status(&runner(&[&bin, "auth", "status", "--json"]).stdout)
    } else {
        AuthStatus::Error {
            code: "cli_unavailable".into(),
            detail: String::new(),
        }
    };
    let cache = read_last_deploy_cache();
    let (auth_ok, auth_error_code, scopes, user_email, expires_at) = match auth_status {
        AuthStatus::Ok {
            user_email,
            expires_at,
            scopes,
            ..
        } => (true, None, scopes, Some(user_email), Some(expires_at)),
        AuthStatus::Error { code, .. } => (false, Some(code), vec![], None, None),
    };
    let output = PreflightOutput {
        cli_version,
        in_range,
        cli_too_old: too_old,
        cli_too_new: too_new,
        cli_present,
        auth_ok,
        auth_error_code,
        scopes,
        profile: std::env::var("AXHUB_PROFILE")
            .ok()
            .filter(|s| !s.is_empty()),
        endpoint: std::env::var("AXHUB_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty()),
        user_email,
        expires_at,
        current_app: std::env::var("AXHUB_APP_SLUG")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| cache.as_ref().and_then(|c| c.app_slug.clone())),
        current_env: std::env::var("AXHUB_PROFILE")
            .ok()
            .filter(|s| !s.is_empty()),
        last_deploy_id: cache.as_ref().map(|c| c.deployment_id.clone()),
        last_deploy_status: cache.as_ref().map(|c| c.status.clone()),
        plugin_version: std::env::var("AXHUB_PLUGIN_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").into()),
    };
    let exit_code = if !cli_present || !in_range {
        EXIT_USAGE
    } else if !output.auth_ok {
        EXIT_AUTH
    } else {
        EXIT_OK
    };
    PreflightRun { output, exit_code }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_drops_prerelease_and_build_like_ts() {
        assert_eq!(
            extract_semver("axhub 1.2.3-rc.1+build"),
            Some("1.2.3".into())
        );
    }
}

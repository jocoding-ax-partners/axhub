use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use chrono::FixedOffset;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};

pub const MIN_AXHUB_CLI_VERSION: &str = "0.1.0";
pub const MAX_AXHUB_CLI_VERSION: &str = "0.13.0";
pub const EXIT_OK: i32 = 0;
pub const EXIT_USAGE: i32 = 64;
pub const EXIT_AUTH: i32 = 65;

static SEMVER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)\.(\d+)\.(\d+)").unwrap());
static APP_SLUG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-]*$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn default_runner(cmd: &[&str]) -> SpawnResult {
    // Production-spawn path resolution: when cmd[0] is the bare axhub basename
    // ("axhub" / "axhub.exe"), substitute the resolved absolute path discovered via
    // PATH search + well-known fallback dirs (Apple Silicon Homebrew, cargo bin,
    // sh-native `~/.local/bin`, etc.). macOS GUI subprocesses inherit a stripped PATH
    // that misses these locations, which is why pre-v0.9.4 preflight reported
    // `cli_present: false` for users who had installed via Homebrew. Test paths use
    // mock runners and never enter this branch, so integration mocks still match
    // ["axhub", ...] literals.
    let resolved;
    let effective: Vec<&str> = if !cmd.is_empty()
        && (cmd[0] == "axhub" || cmd[0] == "axhub.exe")
    {
        match resolve_axhub_path() {
            Some(path) => {
                resolved = path;
                std::iter::once(resolved.as_str())
                    .chain(cmd.iter().skip(1).copied())
                    .collect()
            }
            None => cmd.to_vec(),
        }
    } else {
        cmd.to_vec()
    };
    match crate::spawn::spawn_sync(&effective) {
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

/// Logical name used in mock-runner pattern matching (preflight integration tests).
/// Production callers always go through `default_runner` which substitutes the resolved
/// absolute path right before `Command::new(...)` so real spawns can find the binary
/// outside the inherited PATH (e.g. macOS GUI subprocess that lacks `/opt/homebrew/bin`).
pub fn axhub_bin() -> String {
    std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string())
}

#[cfg(windows)]
pub const AXHUB_BIN_NAME: &str = "axhub.exe";
#[cfg(not(windows))]
pub const AXHUB_BIN_NAME: &str = "axhub";

/// Search for `axhub` binary on PATH plus well-known OS fallback locations.
///
/// macOS GUI app subprocesses (incl. Claude Code Desktop) don't inherit shell-profile
/// PATH additions like `/opt/homebrew/bin` (Apple Silicon Homebrew, default since 2020)
/// or `~/.cargo/bin` (cargo install). When `axhub` is installed via Homebrew or cargo,
/// the inherited PATH may not contain the install directory, so `Command::new("axhub")`
/// fails with "command not found" even though the binary is on disk. This fallback
/// search supplements PATH with common install locations so preflight reports
/// `cli_present: true` when the CLI is reachable from any standard install path.
///
/// Returns the first absolute path that exists, else `None` (caller keeps bare basename
/// — spawn proceeds with PATH semantics so test mocks that match `["axhub", ...]` work).
pub fn resolve_axhub_path() -> Option<String> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(AXHUB_BIN_NAME);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    for candidate in fallback_axhub_paths() {
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

pub fn fallback_axhub_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "macos") {
        paths.push(PathBuf::from("/opt/homebrew/bin").join(AXHUB_BIN_NAME));
        paths.push(PathBuf::from("/usr/local/bin").join(AXHUB_BIN_NAME));
    } else if cfg!(target_os = "linux") {
        paths.push(PathBuf::from("/usr/local/bin").join(AXHUB_BIN_NAME));
        paths.push(PathBuf::from("/usr/bin").join(AXHUB_BIN_NAME));
        paths.push(PathBuf::from("/home/linuxbrew/.linuxbrew/bin").join(AXHUB_BIN_NAME));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    if let Some(home) = home {
        paths.push(home.join(".cargo/bin").join(AXHUB_BIN_NAME));
        paths.push(home.join(".local/bin").join(AXHUB_BIN_NAME));
    }
    paths
}

/// Returns `true` when `AXHUB_PERF_AUTO_APPROVE=1` is set.
///
/// Test/CI only — never set in user production. Used by the perf walltime
/// test suite (Phase 0) to bypass `AskUserQuestion` consent so that walltime
/// excludes user think time. Production flows ignore this signal entirely.
pub fn auto_approve_enabled() -> bool {
    std::env::var("AXHUB_PERF_AUTO_APPROVE").as_deref() == Ok("1")
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
    pub expires_human: Option<String>,
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
    crate::runtime_paths::last_deploy_file()
        .unwrap_or_else(|| home_dir().join(".cache/axhub-plugin/last-deploy.json"))
}

fn read_last_deploy_cache() -> Option<LastDeployCache> {
    serde_json::from_str(&fs::read_to_string(last_deploy_cache_path()).ok()?).ok()
}

fn parse_manifest_app_slug(raw: &str) -> Option<String> {
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
            if APP_SLUG_RE.is_match(value) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn read_manifest_current_app() -> Option<String> {
    ["apphub.yaml", "axhub.yaml"].into_iter().find_map(|path| {
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| parse_manifest_app_slug(&raw))
    })
}

/// Returns true when the current working directory contains a project-marker
/// file or directory. Used by `current_app` resolution to gate cache fallback —
/// an empty cwd (or a non-project directory like `~/Downloads`) should NOT
/// inherit the `app_slug` from the most-recent global deploy cache, because
/// SKILL routing reads `current_app` and renders "현재 앱: <slug>" prompts.
/// v0.9.5 fixes a regression where cache was emitted unconditionally and
/// users saw stale "현재 앱: nextjs-axhub" in unrelated empty directories.
fn cwd_has_project_marker() -> bool {
    const MARKERS: &[&str] = &[
        "apphub.yaml",
        "axhub.yaml",
        ".git",
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "go.mod",
        "deno.json",
        "deno.jsonc",
        "Gemfile",
        "composer.json",
        "build.gradle",
        "build.gradle.kts",
        "pom.xml",
    ];
    MARKERS.iter().any(|marker| std::path::Path::new(marker).exists())
}

/// Classify the result of `axhub --version` into a discriminated CLI state so
/// downstream SKILL wrappers + systemMessage emitters can route the user to
/// the correct fix (install / re-auth / upgrade / report) instead of the
/// blanket "cli_unavailable" message that v0.9.3 emitted.
fn diagnose_cli_state(version_result: &SpawnResult) -> CliState {
    if version_result.exit_code == EXIT_OK && !version_result.stdout.is_empty() {
        return CliState::Ok;
    }
    let stderr_lower = version_result.stderr.to_lowercase();
    // Exit code 127 = command not found (POSIX). Also catch spawn-error stderr text.
    if version_result.exit_code == 127
        || stderr_lower.contains("command not found")
        || stderr_lower.contains("no such file")
        || stderr_lower.contains("not recognized as the name")
    {
        return CliState::NotFound;
    }
    // Config schema mismatch (e.g. v0.9.4 user_id UUID -> int64 breakage).
    if stderr_lower.contains("load config")
        || stderr_lower.contains("cannot parse value")
        || stderr_lower.contains("config.yaml")
        || stderr_lower.contains("config:")
        || stderr_lower.contains("yaml:")
    {
        return CliState::ConfigCorrupted;
    }
    CliState::RuntimeError
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliState {
    Ok,
    NotFound,
    ConfigCorrupted,
    RuntimeError,
}

impl CliState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NotFound => "not_found",
            Self::ConfigCorrupted => "config_corrupted",
            Self::RuntimeError => "runtime_error",
        }
    }

    /// Subcode string emitted as `auth_error_code` in the preflight JSON when
    /// the CLI cannot be probed. Picked up by SKILL `!command` preflight
    /// wrappers via regex match so they can render a fix-specific systemMessage.
    pub fn auth_error_code(&self) -> Option<&'static str> {
        match self {
            Self::Ok => None,
            Self::NotFound => Some("cli_not_found"),
            Self::ConfigCorrupted => Some("cli_config_corrupted"),
            Self::RuntimeError => Some("cli_runtime_error"),
        }
    }
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
    F: Fn(&[&str]) -> SpawnResult + Sync,
{
    let bin = axhub_bin();
    let parallel_disabled = std::env::var("AXHUB_PREFLIGHT_PARALLEL").as_deref() == Ok("0");

    // Phase 3 B-06: spawn the four independent probes in parallel
    // (version + auth + last-deploy cache + manifest read). The auth
    // probe always spawns; if `--version` fails we override the result
    // post-join because re-running CLI auth without a CLI is irrelevant.
    // AXHUB_PREFLIGHT_PARALLEL=0 falls back to the sequential path below
    // for environments where thread::scope misbehaves.
    let (version_result, raw_auth_status, cache, manifest_app) = if parallel_disabled {
        let version_result = runner(&[&bin, "--version"]);
        let auth_raw = parse_auth_status(&runner(&[&bin, "auth", "status", "--json"]).stdout);
        let cache = read_last_deploy_cache();
        let manifest_app = read_manifest_current_app();
        (version_result, auth_raw, cache, manifest_app)
    } else {
        let runner_ref = &runner;
        let bin_ref: &str = &bin;
        std::thread::scope(|scope| {
            let v_handle = scope.spawn(move || runner_ref(&[bin_ref, "--version"]));
            let a_handle = scope.spawn(move || {
                parse_auth_status(&runner_ref(&[bin_ref, "auth", "status", "--json"]).stdout)
            });
            let c_handle = scope.spawn(read_last_deploy_cache);
            let m_handle = scope.spawn(read_manifest_current_app);
            let v = v_handle.join().expect("version probe thread panicked");
            let a = a_handle.join().expect("auth probe thread panicked");
            let c = c_handle.join().expect("cache probe thread panicked");
            let m = m_handle.join().expect("manifest probe thread panicked");
            (v, a, c, m)
        })
    };

    let cli_state = diagnose_cli_state(&version_result);
    let cli_present = cli_state == CliState::Ok;
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
        raw_auth_status
    } else {
        AuthStatus::Error {
            // v0.9.5: surface specific cli failure mode (not_found / config_corrupted /
            // runtime_error) so SKILL wrappers can render a fix-specific systemMessage
            // instead of the blanket cli_unavailable that conflated install vs. config drift.
            // Backward compat: keep `cli_unavailable` as the auth_error_code value for the
            // RuntimeError/legacy path so existing wrapper regex still matches.
            code: cli_state
                .auth_error_code()
                .unwrap_or("cli_unavailable")
                .into(),
            detail: String::new(),
        }
    };
    let (auth_ok, auth_error_code, scopes, user_email, expires_at) = match auth_status {
        AuthStatus::Ok {
            user_email,
            expires_at,
            scopes,
            ..
        } => (true, None, scopes, Some(user_email), Some(expires_at)),
        AuthStatus::Error { code, .. } => (false, Some(code), vec![], None, None),
    };
    let expires_human = expires_at.as_deref().and_then(|iso| {
        crate::humanize::format_expires_human(
            iso,
            FixedOffset::east_opt(9 * 3600).unwrap(),
            chrono::Utc::now(),
        )
    });
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
        expires_human,
        current_app: std::env::var("AXHUB_APP_SLUG")
            .ok()
            .filter(|s| !s.is_empty())
            .or(manifest_app)
            // v0.9.5: cache.app_slug 은 globally cached "last deploy" 라 cwd context
            // 와 무관하게 emit 되면 빈 디렉토리에서도 "현재 앱: <stale-slug>" 가 떠요.
            // SKILL routing 이 이를 보고 잘못된 안내를 함 (#: 사용자 보고된 회귀).
            // cwd 에 project marker (.git / package.json / apphub.yaml / Cargo.toml 등)
            // 가 있을 때만 cache fallback 활성화. 그 외는 None — SKILL 가 "현재 앱 없음"
            // 으로 graceful 안내.
            .or_else(|| {
                if cwd_has_project_marker() {
                    cache.as_ref().and_then(|c| c.app_slug.clone())
                } else {
                    None
                }
            }),
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

    fn auto_approve_env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[test]
    fn auto_approve_enabled_only_when_env_is_one() {
        let _guard = auto_approve_env_lock().lock().unwrap();
        let prev = std::env::var("AXHUB_PERF_AUTO_APPROVE").ok();

        std::env::remove_var("AXHUB_PERF_AUTO_APPROVE");
        assert!(!auto_approve_enabled(), "unset should be false");

        std::env::set_var("AXHUB_PERF_AUTO_APPROVE", "0");
        assert!(!auto_approve_enabled(), "0 should be false");

        std::env::set_var("AXHUB_PERF_AUTO_APPROVE", "true");
        assert!(!auto_approve_enabled(), "non-1 string should be false");

        std::env::set_var("AXHUB_PERF_AUTO_APPROVE", "1");
        assert!(auto_approve_enabled(), "1 should be true");

        match prev {
            Some(v) => std::env::set_var("AXHUB_PERF_AUTO_APPROVE", v),
            None => std::env::remove_var("AXHUB_PERF_AUTO_APPROVE"),
        }
    }

    #[test]
    fn auth_status_ok_reports_only_authenticated_shape() {
        assert!(AuthStatus::Ok {
            user_email: "dev@example.test".into(),
            user_id: 7,
            expires_at: "2099-01-01T00:00:00Z".into(),
            scopes: vec!["read".into()],
        }
        .ok());

        assert!(!AuthStatus::Error {
            code: "auth.token_missing".into(),
            detail: "missing".into(),
        }
        .ok());
    }

    fn axhub_bin_env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[test]
    fn axhub_bin_honors_explicit_env_override() {
        let _guard = axhub_bin_env_lock().lock().unwrap();
        let prev = std::env::var("AXHUB_BIN").ok();
        std::env::set_var("AXHUB_BIN", "/custom/path/to/axhub");
        assert_eq!(axhub_bin(), "/custom/path/to/axhub");
        std::env::remove_var("AXHUB_BIN");
        match prev {
            Some(v) => std::env::set_var("AXHUB_BIN", v),
            None => std::env::remove_var("AXHUB_BIN"),
        }
    }

    #[test]
    fn fallback_paths_include_apple_silicon_homebrew_on_macos() {
        let paths = fallback_axhub_paths();
        let path_strings: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if cfg!(target_os = "macos") {
            assert!(
                path_strings.iter().any(|p| p == "/opt/homebrew/bin/axhub"),
                "macOS fallback list missing Apple Silicon Homebrew path: {:?}",
                path_strings
            );
            assert!(
                path_strings.iter().any(|p| p == "/usr/local/bin/axhub"),
                "macOS fallback list missing Intel Homebrew / sh-native path: {:?}",
                path_strings
            );
        }
        if cfg!(target_os = "linux") {
            assert!(
                path_strings.iter().any(|p| p == "/usr/local/bin/axhub"),
                "Linux fallback list missing /usr/local/bin path: {:?}",
                path_strings
            );
        }
    }

    #[test]
    fn fallback_paths_include_cargo_and_local_bin_when_home_set() {
        let _guard = axhub_bin_env_lock().lock().unwrap();
        let prev_home = std::env::var("HOME").ok();
        let prev_user = std::env::var("USERPROFILE").ok();
        std::env::set_var("HOME", "/tmp/fake-home-for-test");
        std::env::remove_var("USERPROFILE");

        let paths = fallback_axhub_paths();
        let path_strings: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert!(
            path_strings
                .iter()
                .any(|p| p.contains("/tmp/fake-home-for-test/.cargo/bin/axhub")),
            "fallback list missing cargo bin under HOME: {:?}",
            path_strings
        );
        assert!(
            path_strings
                .iter()
                .any(|p| p.contains("/tmp/fake-home-for-test/.local/bin/axhub")),
            "fallback list missing $HOME/.local/bin (sh-native non-root path): {:?}",
            path_strings
        );

        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        if let Some(v) = prev_user {
            std::env::set_var("USERPROFILE", v);
        }
    }

    #[test]
    fn resolve_axhub_path_finds_binary_in_path() {
        let _guard = axhub_bin_env_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "axhub-resolve-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let fake = dir.join(AXHUB_BIN_NAME);
        fs::write(&fake, b"#!/bin/sh\necho axhub 0.12.0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = fs::metadata(&fake).unwrap().permissions();
            perm.set_mode(0o755);
            fs::set_permissions(&fake, perm).unwrap();
        }

        let prev_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", dir.to_string_lossy().to_string());

        let resolved = resolve_axhub_path();
        assert_eq!(resolved.as_deref(), Some(fake.to_string_lossy().as_ref()));

        match prev_path {
            Some(v) => std::env::set_var("PATH", v),
            None => std::env::remove_var("PATH"),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}

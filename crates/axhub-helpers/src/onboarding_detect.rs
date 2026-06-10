//! `onboarding-detect` — single cross-platform read-only scan that replaces the
//! onboarding SKILL's dual bash/PowerShell `DETECT_ALL` blocks.
//!
//! Why a helper subcommand: the inline DETECT_ALL scripts were large enough that
//! the model narrated a condensed copy into the user-facing chat before running
//! them (the leaked `$minAxhubCliVersion … 온보딩 상태 확인` block). Collapsing the
//! whole scan into one opaque `axhub-helpers onboarding-detect --json` call — the
//! same shape as `preflight` — leaves nothing in the SKILL body to narrate, so the
//! leak is structurally impossible.
//!
//! It also fixes the GitHub surface:
//!   * `install_url` is read from `accounts[].install_url` (there is no top-level
//!     field), so it is populated whenever any account exists and the SKILL can
//!     show it **unconditionally — even for already-installed users** who want to
//!     add another org/account.
//!   * an auth-error envelope (`{"status":"error","error":{"code":"auth"}}`) maps
//!     to a distinct `auth_error` state instead of silently collapsing to
//!     `unknown`, so the SKILL routes to re-login instead of claiming it "can't
//!     check".
//!
//! Fail-open contract: every probe degrades to a safe default, `run()` never
//! panics, and the dispatcher always exits 0.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::axhub_cli::{axhub_bin_from_env, run_axhub, run_axhub_with_timeout};
use crate::preflight::{run_preflight, CliState};

pub const SCHEMA_VERSION: &str = "onboarding-detect/v1";

/// App-level GitHub App install page, used as a last-resort fallback so the
/// "always show install_url" contract holds even when `accounts list` returns a
/// successful-but-empty list (an authenticated user who has the App installed
/// nowhere — the person who most needs the link). Every per-account `install_url`
/// observed is this exact URL. Applied ONLY to a successful (`status:ok`)
/// response; auth/transport errors stay `None` so the SKILL routes to re-login
/// first. Defaults to the `ax-hub-deploy` SaaS app but is overridable via
/// `AXHUB_GITHUB_APP_INSTALL_URL` for self-hosted / non-default backends.
const DEFAULT_INSTALL_URL: &str = "https://github.com/apps/ax-hub-deploy/installations/new";

/// The install-url fallback, honoring the `AXHUB_GITHUB_APP_INSTALL_URL` override
/// (only when it is a valid github.com URL).
fn default_install_url() -> String {
    std::env::var("AXHUB_GITHUB_APP_INSTALL_URL")
        .ok()
        .filter(|u| is_github_install_url(u))
        .unwrap_or_else(|| DEFAULT_INSTALL_URL.to_string())
}

/// Guard a surfaced install link to an `https://github.com/` URL. The SKILL
/// renders this as a clickable install link, so a compromised/misconfigured
/// backend (or env override) must not be able to point it at an arbitrary host.
fn is_github_install_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
}

/// Normalized GitHub App install state. Serializes to the same lowercase strings
/// the SKILL switches on (`installed`/`mixed`/`uninstalled`/`empty`/`auth_error`/
/// `unavailable`) — a closed set instead of a stringly-typed field.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GithubStateKind {
    Installed,
    Mixed,
    Uninstalled,
    Empty,
    AuthError,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GithubState {
    pub state: GithubStateKind,
    pub installed_logins: Vec<String>,
    pub uninstalled_logins: Vec<String>,
    /// App-level install/add-account URL. All accounts share the same value, so
    /// it is populated whenever any account carries it (validated to github.com).
    /// The SKILL surfaces this in every state (including `installed`) per the
    /// "always show install_url" contract.
    pub install_url: Option<String>,
    /// `true` when ≥2 accounts have the App installed — bootstrap then needs
    /// `--github-owner`/`--installation-id` or it fails with `ambiguous_installation`.
    pub multiple_installed: bool,
}

impl GithubState {
    fn unavailable() -> Self {
        Self {
            state: GithubStateKind::Unavailable,
            installed_logins: Vec::new(),
            uninstalled_logins: Vec::new(),
            install_url: None,
            multiple_installed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OnboardingDetect {
    pub schema_version: String,
    // CLI / auth (reused from preflight).
    pub cli_present: bool,
    pub cli_version: Option<String>,
    pub cli_state: String,
    pub cli_on_path: bool,
    /// Resolved path of the healthy CLI; when `cli_state` is `axhub_bin_invalid`
    /// it instead carries the dead `AXHUB_BIN` override value (see preflight.rs).
    pub cli_resolved_path: Option<String>,
    pub cli_too_old: bool,
    pub has_update: bool,
    pub latest_version: Option<String>,
    pub auth_ok: bool,
    pub auth_error_code: Option<String>,
    // Local environment.
    pub git_present: bool,
    pub git_repo: bool,
    pub git_commit: bool,
    pub node_present: bool,
    pub node_version: Option<String>,
    pub node_required: Option<String>,
    pub node_mismatch: bool,
    pub manifest_present: bool,
    pub lockfile_present: bool,
    pub deps_missing: bool,
    pub dir_empty: bool,
    // GitHub App install surface.
    pub github: GithubState,
    // Deploy verification (only when bootstrap state exists).
    pub deploy_checked: bool,
    pub deploy_verified: bool,
    // First actionable gap (state-machine order) + full ordered list.
    pub first_gap: Option<String>,
    pub gaps: Vec<String>,
}

/// Parse `axhub github accounts list --json` into a normalized state.
///
/// Bug-2 core: distinguishes an auth-error envelope from genuinely empty/missing
/// accounts, and always extracts `install_url` from `accounts[]` (where it
/// actually lives — there is no top-level `install_url`).
pub fn parse_github_state(stdout: &str, timed_out: bool) -> GithubState {
    if timed_out || stdout.trim().is_empty() {
        return GithubState::unavailable();
    }
    let Ok(v) = serde_json::from_str::<Value>(stdout) else {
        return GithubState::unavailable();
    };

    // Error envelope: `{"status":"error","error":{"code":"auth", ...}}`.
    if v.get("status").and_then(Value::as_str) == Some("error") {
        let code = v
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str);
        return GithubState {
            state: if code == Some("auth") {
                GithubStateKind::AuthError
            } else {
                GithubStateKind::Unavailable
            },
            ..GithubState::unavailable()
        };
    }

    let accounts = v
        .get("data")
        .and_then(|d| d.get("accounts"))
        .and_then(Value::as_array)
        .or_else(|| v.get("accounts").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();

    let mut installed_logins = Vec::new();
    let mut uninstalled_logins = Vec::new();
    let mut installed_count = 0usize;
    let mut uninstalled_count = 0usize;
    let mut install_url: Option<String> = None;

    for account in &accounts {
        if install_url.is_none() {
            // Only surface a github.com URL — never an arbitrary host from a
            // misconfigured/compromised backend (the SKILL renders it clickable).
            if let Some(url) = account
                .get("install_url")
                .and_then(Value::as_str)
                .filter(|u| is_github_install_url(u))
            {
                install_url = Some(url.to_string());
            }
        }
        let installed = account.get("installed").and_then(Value::as_bool) == Some(true)
            || account.get("installation_id").is_some_and(|x| !x.is_null())
            || account.get("installationId").is_some_and(|x| !x.is_null());
        // Count every account by install state — `multiple_installed` and the
        // state discriminant must not under-count a login-less installed account
        // (else the SKILL skips the owner-pick and bootstrap still hits exit-9).
        if installed {
            installed_count += 1;
        } else {
            uninstalled_count += 1;
        }
        if let Some(login) = account.get("login").and_then(Value::as_str) {
            if installed {
                installed_logins.push(login.to_string());
            } else {
                uninstalled_logins.push(login.to_string());
            }
        }
    }

    let state = if accounts.is_empty() {
        GithubStateKind::Empty
    } else if installed_count > 0 && uninstalled_count > 0 {
        GithubStateKind::Mixed
    } else if installed_count > 0 {
        GithubStateKind::Installed
    } else {
        GithubStateKind::Uninstalled
    };

    GithubState {
        multiple_installed: installed_count >= 2,
        state,
        installed_logins,
        uninstalled_logins,
        // Successful response → always expose an install_url, falling back to the
        // app-level page (env-overridable) when no account carried a valid one —
        // e.g. an empty list for a brand-new user. Honors "always show install_url".
        install_url: install_url.or_else(|| Some(default_install_url())),
    }
}

/// Run a local command and return trimmed stdout on success (empty allowed),
/// `None` on spawn failure or non-zero exit.
fn command_output(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn any_exists(names: &[&str]) -> bool {
    names.iter().any(|n| Path::new(n).exists())
}

/// Lockfiles the onboarding flow recognizes (any present ⇒ deps installable).
const LOCKFILES: [&str; 5] = [
    "bun.lockb",
    "bun.lock",
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
];

const MANIFESTS: [&str; 2] = ["axhub.yaml", "apphub.yaml"];

/// Entries ignored when deciding whether the directory is "empty" for onboarding.
const DIR_IGNORE: [&str; 6] = [
    ".git",
    "node_modules",
    ".omc",
    ".claude",
    ".axhub-state",
    ".DS_Store",
];

fn dir_is_empty() -> bool {
    dir_is_empty_at(Path::new("."))
}

/// `true` when `dir` contains nothing but ignorable scaffolding (`DIR_IGNORE`).
/// Split from the CWD entry point so it is unit-testable against a temp dir.
fn dir_is_empty_at(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        // Can't read the dir — treat as non-empty (conservative: don't offer fresh init).
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !DIR_IGNORE.contains(&name.as_ref()) {
            return false;
        }
    }
    true
}

fn read_node_required() -> Option<String> {
    if let Ok(nvmrc) = std::fs::read_to_string(".nvmrc") {
        let line = nvmrc.lines().next().unwrap_or("").trim();
        if !line.is_empty() {
            return Some(line.to_string());
        }
    }
    let pkg = std::fs::read_to_string("package.json").ok()?;
    let v: Value = serde_json::from_str(&pkg).ok()?;
    v.get("engines")
        .and_then(|e| e.get("node"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn major_of(version: &str) -> Option<u64> {
    version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// Conservative node-version match: only flags a mismatch when the requirement
/// expresses a clear major constraint. Anything ambiguous (complex ranges, OR
/// clauses, unparseable) is treated as a match — a false "ok" is safer here than
/// a false "fix your node" prompt during onboarding.
fn node_mismatch(active: &str, required: &str) -> bool {
    let Some(active_major) = major_of(active) else {
        return false;
    };
    let req = required.trim();
    // `>=18`, `>18`, `>= 18.0.0` → at-least constraint.
    if let Some(rest) = req.strip_prefix(">=").or_else(|| req.strip_prefix('>')) {
        return match major_of(rest) {
            Some(min) => active_major < min,
            None => false,
        };
    }
    // Bare pin: `20`, `20.x`, `v20`, `20.1.0` → exact major.
    if req
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == 'v' || c == 'V')
        && !req.contains([' ', '|', '<', '-'])
    {
        return match major_of(req) {
            Some(want) => active_major != want,
            None => false,
        };
    }
    false
}

fn detect_has_update(cli_present: bool) -> (bool, Option<String>) {
    if !cli_present {
        return (false, None);
    }
    parse_update_check(&run_axhub(&["update", "check", "--json"]).stdout)
}

/// Parse `axhub update check --json` → (`has_update`, `latest_version`).
/// Pure so it is covered without a real CLI on every platform.
fn parse_update_check(stdout: &str) -> (bool, Option<String>) {
    let Ok(v) = serde_json::from_str::<Value>(stdout) else {
        return (false, None);
    };
    let has_update = v
        .get("has_update")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let latest = v
        .get("latest")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    (has_update, latest)
}

/// (`deploy_checked`, `deploy_verified`) from `.axhub/bootstrap.state.json`.
fn detect_deploy(cli_present: bool, auth_ok: bool) -> (bool, bool) {
    if !cli_present || !auth_ok {
        return (false, false);
    }
    let Ok(raw) = std::fs::read_to_string(".axhub/bootstrap.state.json") else {
        return (false, false);
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return (false, false);
    };
    let app_id = v
        .get("app_id")
        .or_else(|| v.get("appId"))
        .and_then(Value::as_str);
    let deploy_id = v
        .get("last_deploy_id")
        .or_else(|| v.get("deployment_id"))
        .or_else(|| v.get("deploymentId"))
        .and_then(Value::as_str);
    let (Some(app_id), Some(deploy_id)) = (app_id, deploy_id) else {
        return (false, false);
    };
    // This call carries `--watch-timeout 1m`, so it must NOT inherit the default
    // 5s probe cap (which would kill the child first and report every in-progress
    // deploy as unverified). Give it 65s — just past the watch budget.
    let out = run_axhub_with_timeout(
        &axhub_bin_from_env(),
        &[
            "deploy",
            "status",
            deploy_id,
            "--app",
            app_id,
            "--watch",
            "--watch-timeout",
            "1m",
            "--json",
        ],
        Duration::from_secs(65),
    );
    (true, parse_deploy_verified(&out.stdout))
}

/// Parse `axhub deploy status --json` → whether the deploy is live. Pure so it
/// is covered without a real CLI on every platform.
fn parse_deploy_verified(stdout: &str) -> bool {
    serde_json::from_str::<Value>(stdout)
        .ok()
        .map(|v| {
            let status = v
                .get("status")
                .or_else(|| v.get("data").and_then(|d| d.get("status")))
                .or_else(|| v.get("deployment").and_then(|d| d.get("status")))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            // Exact match, not substring — a status like `not_running` or
            // `deploy_failed` must not false-positive on `running`/`deployed`.
            ["succeeded", "live", "running", "deployed"].contains(&status.as_str())
        })
        .unwrap_or(false)
}

/// Ordered gap list mirroring the onboarding SKILL Step 3 state machine. The
/// first element is the gap to act on; the SKILL re-runs detect after closing it.
fn compute_gaps(d: &OnboardingDetect) -> Vec<String> {
    let mut gaps = Vec::new();
    if !d.cli_present {
        if d.cli_state == CliState::BinOverrideInvalid.as_str() {
            // AXHUB_BIN override is broken — reinstalling won't help, so route
            // to an env-fix gap instead of the install flow.
            gaps.push("cli_env_invalid".to_string());
        } else {
            gaps.push("cli_missing".to_string());
        }
    } else if !d.cli_on_path {
        gaps.push("cli_path_missing".to_string());
    }
    if d.cli_too_old || d.has_update {
        gaps.push("cli_old".to_string());
    }
    if d.cli_present && !d.auth_ok {
        gaps.push("auth_missing".to_string());
    }
    if !d.git_present {
        gaps.push("git_missing".to_string());
    }
    if !d.node_present {
        gaps.push("node_missing".to_string());
    }
    if d.node_mismatch {
        gaps.push("node_mismatch".to_string());
    }
    // GitHub gap only when authenticated and no account has the App installed.
    if d.auth_ok
        && matches!(
            d.github.state,
            GithubStateKind::Uninstalled | GithubStateKind::Empty
        )
    {
        gaps.push("github_app_missing".to_string());
    }
    if d.git_repo && d.git_commit && !d.manifest_present {
        gaps.push("existing_repo_gap".to_string());
    } else if !d.manifest_present && d.dir_empty {
        gaps.push("no_manifest_empty".to_string());
    }
    if d.deps_missing {
        gaps.push("deps_missing".to_string());
    }
    if d.deploy_checked && !d.deploy_verified {
        gaps.push("deploy_unverified".to_string());
    }
    gaps
}

/// Run the full read-only scan. Never panics; returns a fully-populated struct.
pub fn detect() -> OnboardingDetect {
    let pf = run_preflight().output;

    let git_present = command_output("git", &["--version"]).is_some();
    let git_repo =
        command_output("git", &["rev-parse", "--is-inside-work-tree"]).as_deref() == Some("true");
    let git_commit = command_output("git", &["rev-parse", "--verify", "HEAD"]).is_some();

    let node_version = command_output("node", &["--version"]);
    let node_present = node_version.is_some();
    let node_required = read_node_required();
    let node_mismatch = match (&node_version, &node_required) {
        (Some(active), Some(required)) => node_mismatch(active, required),
        _ => false,
    };

    let manifest_present = any_exists(&MANIFESTS);
    let lockfile_present = any_exists(&LOCKFILES);
    let deps_missing = manifest_present && lockfile_present && !Path::new("node_modules").exists();
    let dir_empty = dir_is_empty();

    // The three CLI probes (update check / github accounts / deploy status) are
    // independent and each can block on a slow backend (deploy waits up to ~65s).
    // Run them concurrently instead of serially — mirrors run_preflight's parallel
    // probe pattern — so a degraded backend doesn't stall every onboarding turn.
    let auth_ok = pf.auth_ok;
    let ((has_update, latest_version), github, (deploy_checked, deploy_verified)) =
        if pf.cli_present {
            std::thread::scope(|scope| {
                let update = scope.spawn(|| detect_has_update(true));
                let gh = scope.spawn(|| {
                    let out = run_axhub(&["github", "accounts", "list", "--json"]);
                    parse_github_state(&out.stdout, out.timed_out)
                });
                let deploy = scope.spawn(|| detect_deploy(true, auth_ok));
                (
                    update.join().unwrap_or((false, None)),
                    gh.join().unwrap_or_else(|_| GithubState::unavailable()),
                    deploy.join().unwrap_or((false, false)),
                )
            })
        } else {
            ((false, None), GithubState::unavailable(), (false, false))
        };

    let mut detect = OnboardingDetect {
        schema_version: SCHEMA_VERSION.to_string(),
        cli_present: pf.cli_present,
        cli_version: pf.cli_version,
        cli_state: pf.cli_state,
        cli_on_path: pf.cli_on_path,
        cli_resolved_path: pf.cli_resolved_path,
        cli_too_old: pf.cli_too_old,
        has_update,
        latest_version,
        auth_ok: pf.auth_ok,
        auth_error_code: pf.auth_error_code,
        git_present,
        git_repo,
        git_commit,
        node_present,
        node_version,
        node_required,
        node_mismatch,
        manifest_present,
        lockfile_present,
        deps_missing,
        dir_empty,
        github,
        deploy_checked,
        deploy_verified,
        first_gap: None,
        gaps: Vec::new(),
    };
    detect.gaps = compute_gaps(&detect);
    detect.first_gap = detect.gaps.first().cloned();
    detect
}

/// `onboarding-detect` subcommand entry point. Fail-open: always returns 0.
pub fn run() -> anyhow::Result<i32> {
    let result = detect();
    println!("{}", serde_json::to_string(&result)?);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_three_installed_orgs_all_installed_with_url() {
        // The live shape that the user's re-authenticated CLI returns.
        let json = r#"{"schema_version":"1","status":"ok","data":{"accounts":[
            {"login":"realitsyourman","type":"User","installed":true,"installation_id":137870131,"install_url":"https://github.com/apps/ax-hub-deploy/installations/new"},
            {"login":"demodev-lab","type":"Organization","installed":true,"installation_id":134239212,"install_url":"https://github.com/apps/ax-hub-deploy/installations/new"},
            {"login":"jocoding-ax-partners","type":"Organization","installed":true,"installation_id":133340904,"install_url":"https://github.com/apps/ax-hub-deploy/installations/new"}
        ]}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.state, GithubStateKind::Installed);
        assert_eq!(s.installed_logins.len(), 3);
        assert!(s.multiple_installed);
        // install_url must be populated even though everything is installed.
        assert_eq!(
            s.install_url.as_deref(),
            Some("https://github.com/apps/ax-hub-deploy/installations/new")
        );
    }

    #[test]
    fn github_auth_error_is_distinct_from_unknown() {
        // The pre-re-login envelope that previously collapsed to "unknown".
        let json = r#"{"schema_version":"1","status":"error","error":{"code":"auth","category":"auth","hint":"not authenticated"}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.state, GithubStateKind::AuthError);
        assert!(s.install_url.is_none());
    }

    #[test]
    fn github_non_auth_error_is_unavailable() {
        let json = r#"{"status":"error","error":{"code":"server_error"}}"#;
        assert_eq!(
            parse_github_state(json, false).state,
            GithubStateKind::Unavailable
        );
    }

    #[test]
    fn github_uninstalled_still_exposes_install_url() {
        let json = r#"{"status":"ok","data":{"accounts":[
            {"login":"solo","installed":false,"install_url":"https://github.com/apps/ax-hub-deploy/installations/new"}
        ]}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.state, GithubStateKind::Uninstalled);
        assert!(!s.multiple_installed);
        assert_eq!(s.uninstalled_logins, vec!["solo".to_string()]);
        assert!(s.install_url.is_some());
    }

    #[test]
    fn github_mixed_install_states() {
        let json = r#"{"status":"ok","data":{"accounts":[
            {"login":"a","installed":true,"installation_id":1,"install_url":"u"},
            {"login":"b","installed":false,"install_url":"u"}
        ]}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.state, GithubStateKind::Mixed);
        assert!(!s.multiple_installed);
        assert_eq!(s.installed_logins, vec!["a".to_string()]);
        assert_eq!(s.uninstalled_logins, vec!["b".to_string()]);
    }

    #[test]
    fn github_empty_account_list_falls_back_to_install_url() {
        // A successful-but-empty list (brand-new user, App installed nowhere)
        // still exposes the app-level install_url so the SKILL always has a
        // connect link — the "무조건 install_url" contract.
        let json = r#"{"status":"ok","data":{"accounts":[]}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.state, GithubStateKind::Empty);
        assert_eq!(s.install_url.as_deref(), Some(DEFAULT_INSTALL_URL));
    }

    #[test]
    fn github_empty_or_timeout_stdout_is_unavailable() {
        assert_eq!(
            parse_github_state("", false).state,
            GithubStateKind::Unavailable
        );
        assert_eq!(
            parse_github_state("anything", true).state,
            GithubStateKind::Unavailable
        );
        assert_eq!(
            parse_github_state("not json", false).state,
            GithubStateKind::Unavailable
        );
    }

    #[test]
    fn github_installed_account_without_login_still_counts() {
        // An installed account missing `login` must still count toward
        // `multiple_installed` (else the SKILL skips owner-pick and bootstrap
        // still hits exit-9 ambiguous_installation).
        let json = r#"{"status":"ok","data":{"accounts":[
            {"installed":true,"installation_id":1,"install_url":"https://github.com/apps/x/installations/new"},
            {"login":"b","installed":true,"installation_id":2}
        ]}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.state, GithubStateKind::Installed);
        assert!(
            s.multiple_installed,
            "2 installed (one login-less) → multiple"
        );
        assert_eq!(s.installed_logins, vec!["b".to_string()]);
    }

    #[test]
    fn github_non_github_install_url_is_rejected() {
        // A non-github.com install_url (misconfigured/compromised backend) is
        // dropped; the validated app-level fallback is surfaced instead.
        let json = r#"{"status":"ok","data":{"accounts":[
            {"login":"a","installed":false,"install_url":"https://evil.example.com/phish"}
        ]}}"#;
        let s = parse_github_state(json, false);
        assert_eq!(s.install_url.as_deref(), Some(DEFAULT_INSTALL_URL));
    }

    #[test]
    fn is_github_install_url_guards_host() {
        assert!(is_github_install_url(
            "https://github.com/apps/x/installations/new"
        ));
        assert!(!is_github_install_url("https://evil.example.com/x"));
        assert!(!is_github_install_url("http://github.com/x"));
        assert!(!is_github_install_url("javascript:alert(1)"));
    }

    #[test]
    fn default_install_url_honors_valid_env_override_only() {
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // SAFETY: env mutation serialized by PROCESS_ENV_LOCK (crate-wide).
        unsafe {
            std::env::set_var(
                "AXHUB_GITHUB_APP_INSTALL_URL",
                "https://github.com/apps/custom/installations/new",
            );
        }
        assert_eq!(
            default_install_url(),
            "https://github.com/apps/custom/installations/new"
        );
        // A non-github override is rejected → constant default.
        unsafe {
            std::env::set_var("AXHUB_GITHUB_APP_INSTALL_URL", "https://evil.example.com/x");
        }
        assert_eq!(default_install_url(), DEFAULT_INSTALL_URL);
        unsafe {
            std::env::remove_var("AXHUB_GITHUB_APP_INSTALL_URL");
        }
    }

    #[test]
    fn probes_short_circuit_without_cli() {
        // The cli-gated probes must early-return safe defaults when the CLI is
        // absent (the realistic CI environment, where the parallel scope is skipped).
        assert_eq!(detect_has_update(false), (false, None));
        assert_eq!(detect_deploy(false, true), (false, false));
        assert_eq!(detect_deploy(true, false), (false, false));
    }

    #[test]
    fn dir_is_empty_at_ignores_scaffolding() {
        let base = std::env::temp_dir().join(format!("ax-onb-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("mk tmp");
        assert!(dir_is_empty_at(&base), "fresh temp dir is empty");
        // Only ignorable scaffolding → still empty.
        std::fs::create_dir_all(base.join(".git")).expect("mk .git");
        std::fs::write(base.join(".DS_Store"), b"x").expect("write .DS_Store");
        assert!(dir_is_empty_at(&base), "only ignorable entries → empty");
        // A real file → non-empty.
        std::fs::write(base.join("README.md"), b"hi").expect("write README");
        assert!(!dir_is_empty_at(&base), "a real file → non-empty");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn node_mismatch_exact_major() {
        assert!(node_mismatch("v18.20.0", "20"));
        assert!(!node_mismatch("v20.16.0", "20"));
        assert!(!node_mismatch("v20.16.0", "20.x"));
    }

    #[test]
    fn node_mismatch_at_least() {
        assert!(node_mismatch("v16.0.0", ">=18"));
        assert!(!node_mismatch("v20.0.0", ">=18"));
    }

    #[test]
    fn node_mismatch_ambiguous_is_ok() {
        // Complex ranges / unparseable → no mismatch (conservative).
        assert!(!node_mismatch("v20.0.0", "^18 || ^20"));
        assert!(!node_mismatch("v20.0.0", ""));
        assert!(!node_mismatch("garbage", "20"));
    }

    #[test]
    fn gaps_cli_missing_is_first() {
        let mut d = bare_detect();
        d.cli_present = false;
        let gaps = compute_gaps(&d);
        assert_eq!(gaps.first().map(String::as_str), Some("cli_missing"));
    }

    #[test]
    fn gaps_axhub_bin_invalid_routes_to_env_gap_not_install() {
        let mut d = bare_detect();
        d.cli_present = false;
        d.cli_state = "axhub_bin_invalid".to_string();
        d.git_present = false;
        let gaps = compute_gaps(&d);
        assert_eq!(
            gaps,
            vec!["cli_env_invalid".to_string(), "git_missing".to_string()]
        );
        assert!(!gaps.contains(&"cli_missing".to_string()));
    }

    #[test]
    fn gaps_no_gap_when_all_green() {
        let d = bare_detect();
        assert!(compute_gaps(&d).is_empty());
    }

    #[test]
    fn gaps_github_only_when_authed_and_uninstalled() {
        let mut d = bare_detect();
        d.github = GithubState {
            state: GithubStateKind::Uninstalled,
            ..GithubState::unavailable()
        };
        assert!(compute_gaps(&d).contains(&"github_app_missing".to_string()));
        // Installed → no github gap.
        d.github = GithubState {
            state: GithubStateKind::Installed,
            ..GithubState::unavailable()
        };
        assert!(!compute_gaps(&d).contains(&"github_app_missing".to_string()));
    }

    #[test]
    fn command_output_present_and_absent() {
        // `git --version` resolves on every CI runner.
        assert!(command_output("git", &["--version"]).is_some());
        // A binary that cannot exist resolves to None (spawn failure).
        assert!(command_output("axhub-no-such-binary-zzz", &["--version"]).is_none());
    }

    #[test]
    fn major_of_parses_versions() {
        assert_eq!(major_of("v20.16.1"), Some(20));
        assert_eq!(major_of("18"), Some(18));
        assert_eq!(major_of(" v22 "), Some(22));
        assert_eq!(major_of("garbage"), None);
    }

    #[test]
    fn any_exists_detects_present_and_absent() {
        // cargo runs tests with CWD = crate root, which always has Cargo.toml.
        assert!(any_exists(&["Cargo.toml"]));
        assert!(!any_exists(&["definitely-not-a-real-file-xyz-123"]));
    }

    #[test]
    fn detect_runs_without_panic_and_is_fail_open() {
        // End-to-end read-only scan. Exercises command_output (git/node),
        // dir_is_empty, read_node_required, run_preflight, the update/deploy
        // probes, and compute_gaps. Must never panic regardless of environment.
        let d = detect();
        assert_eq!(d.schema_version, SCHEMA_VERSION);
        assert!(d.gaps.len() <= 13);
        // first_gap is the head of the ordered gap list (or None when clean).
        assert_eq!(d.first_gap.as_deref(), d.gaps.first().map(String::as_str));
        // github.state serializes to one of the SKILL-facing strings (locks the
        // serde rename → the JSON contract the SKILL switches on).
        let state_json = serde_json::to_string(&d.github.state).unwrap_or_default();
        assert!(
            [
                "\"installed\"",
                "\"mixed\"",
                "\"uninstalled\"",
                "\"empty\"",
                "\"auth_error\"",
                "\"unavailable\"",
            ]
            .contains(&state_json.as_str()),
            "unexpected github.state serialization: {state_json}"
        );
    }

    #[test]
    fn run_emits_json_and_exits_zero() {
        // Fail-open contract: the subcommand always returns 0.
        assert_eq!(run().unwrap(), 0);
    }

    #[test]
    fn parse_update_check_variants() {
        assert_eq!(
            parse_update_check(r#"{"has_update":true,"latest":"v0.18.0"}"#),
            (true, Some("v0.18.0".to_string()))
        );
        assert_eq!(parse_update_check(r#"{"has_update":false}"#), (false, None));
        // Empty latest collapses to None.
        assert_eq!(
            parse_update_check(r#"{"has_update":true,"latest":""}"#),
            (true, None)
        );
        // Empty / malformed stdout fails open.
        assert_eq!(parse_update_check(""), (false, None));
        assert_eq!(parse_update_check("not json"), (false, None));
    }

    #[test]
    fn parse_deploy_verified_variants() {
        assert!(parse_deploy_verified(r#"{"status":"succeeded"}"#));
        assert!(parse_deploy_verified(r#"{"data":{"status":"Live"}}"#));
        assert!(parse_deploy_verified(
            r#"{"deployment":{"status":"running"}}"#
        ));
        assert!(!parse_deploy_verified(r#"{"status":"failed"}"#));
        assert!(!parse_deploy_verified(r#"{"status":"building"}"#));
        assert!(!parse_deploy_verified("not json"));
    }

    /// All-green baseline (CLI present, authed, repo+manifest present, github
    /// installed) so individual gap tests flip exactly one field.
    fn bare_detect() -> OnboardingDetect {
        OnboardingDetect {
            schema_version: SCHEMA_VERSION.to_string(),
            cli_present: true,
            cli_version: Some("0.18.0".to_string()),
            cli_state: "ok".to_string(),
            cli_on_path: true,
            cli_resolved_path: Some("/usr/local/bin/axhub".to_string()),
            cli_too_old: false,
            has_update: false,
            latest_version: None,
            auth_ok: true,
            auth_error_code: None,
            git_present: true,
            git_repo: true,
            git_commit: true,
            node_present: true,
            node_version: Some("v20.0.0".to_string()),
            node_required: None,
            node_mismatch: false,
            manifest_present: true,
            lockfile_present: true,
            deps_missing: false,
            dir_empty: false,
            github: GithubState {
                state: GithubStateKind::Installed,
                ..GithubState::unavailable()
            },
            deploy_checked: false,
            deploy_verified: false,
            first_gap: None,
            gaps: Vec::new(),
        }
    }
}

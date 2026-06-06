use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::{DateTime, SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::bootstrap::{build_dependency_plan, PackageManager, PlanState};

pub const SCAFFOLD_DETECT_SCHEMA_VERSION: &str = "scaffold-detect/v1";
pub const SCAFFOLD_DEV_SCHEMA_VERSION: &str = "scaffold-dev/v1";
pub const SCAFFOLD_DEV_STATE_RELATIVE_PATH: &str = ".axhub/scaffold-dev.json";
const SCAFFOLD_DEV_STATE_TTL_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScaffoldDetect {
    pub schema_version: String,
    pub package_json_present: bool,
    pub lockfile_present: bool,
    pub lockfile_count: u32,
    pub detected_lockfile: Option<String>,
    pub manager: Option<String>,
    pub node_available: bool,
    pub node_modules_present: bool,
    pub dev_script_present: bool,
    pub can_install: bool,
    pub can_start_dev: bool,
    pub install_command: Option<Vec<String>>,
    pub dev_command: Option<Vec<String>>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaffoldDevState {
    pub schema_version: String,
    pub pid: u32,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub command: Vec<String>,
    pub log_path: String,
    pub cwd: String,
    pub started_at: String,
    pub updated_at: String,
}

pub fn run_scaffold_detect(args: &[String]) -> anyhow::Result<i32> {
    parse_json_only("scaffold-detect", args)?;
    let cwd = std::env::current_dir()?;
    println!("{}", serde_json::to_string(&detect_scaffold(&cwd)?)?);
    Ok(0)
}

pub fn run_scaffold_dev(args: &[String]) -> anyhow::Result<i32> {
    let Some((action, rest)) = args.split_first() else {
        eprintln!("axhub-helpers scaffold-dev: missing action");
        return Ok(64);
    };
    parse_json_only("scaffold-dev", rest)?;
    let cwd = std::env::current_dir()?;
    let output = match action.as_str() {
        "start" => scaffold_dev_start(&cwd)?,
        "status" => scaffold_dev_status(&cwd)?,
        "stop" => scaffold_dev_stop(&cwd)?,
        other => {
            eprintln!("axhub-helpers scaffold-dev: unknown action \"{other}\"");
            return Ok(64);
        }
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(0)
}

pub fn detect_scaffold(cwd: &Path) -> anyhow::Result<ScaffoldDetect> {
    let dependency = build_dependency_plan(cwd)?;
    let dev_script_present = read_dev_script(cwd)?.is_some();
    let node_available = node_available();
    let lockfile_present = dependency.lockfile_count > 0;
    let manager = (!dependency.requires_pm_choice)
        .then(|| dependency.manager_candidates.first().copied())
        .flatten();

    let install_command = manager.and_then(|pm| {
        let can_plan_install = dependency.package_json_present
            && lockfile_present
            && node_available
            && matches!(dependency.plan_state, PlanState::DependencyInstallRequired);
        can_plan_install.then(|| install_command_for(pm))
    });
    let dev_command = manager.and_then(|pm| {
        let can_plan_dev = dependency.package_json_present
            && lockfile_present
            && node_available
            && dev_script_present
            && !dependency.requires_pm_choice;
        can_plan_dev.then(|| dev_command_for(pm))
    });

    let can_install = install_command.is_some();
    let can_start_dev = dev_command.is_some();
    let reason = if !dependency.package_json_present {
        "package_json_missing"
    } else if !node_available {
        "node_missing"
    } else if dependency.requires_pm_choice {
        "multiple_lockfiles"
    } else if !lockfile_present {
        "lockfile_missing"
    } else if !dev_script_present {
        "dev_script_missing"
    } else if matches!(dependency.plan_state, PlanState::DependencyAlreadyInstalled) {
        "ready"
    } else {
        "install_required"
    }
    .to_string();

    Ok(ScaffoldDetect {
        schema_version: SCAFFOLD_DETECT_SCHEMA_VERSION.to_string(),
        package_json_present: dependency.package_json_present,
        lockfile_present,
        lockfile_count: dependency.lockfile_count,
        detected_lockfile: dependency.detected_lockfile,
        manager: manager.map(package_manager_name).map(str::to_string),
        node_available,
        node_modules_present: dependency.node_modules_present,
        dev_script_present,
        can_install,
        can_start_dev,
        install_command,
        dev_command,
        reason,
    })
}

fn scaffold_dev_start(cwd: &Path) -> anyhow::Result<Value> {
    if let Some(state) = read_scaffold_dev_state(cwd)? {
        let health = scaffold_dev_health(cwd, &state);
        if health.alive {
            return Ok(json!({
                "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
                "action": "already_running",
                "alive": true,
                "ready": health.ready,
                "pid": state.pid,
                "url": state.url,
                "port": state.port,
                "reason": health.reason
            }));
        }
    }

    let detect = detect_scaffold(cwd)?;
    if !detect.package_json_present
        || !detect.lockfile_present
        || !detect.node_available
        || !detect.dev_script_present
        || detect.dev_command.is_none()
    {
        return Ok(json!({
            "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
            "action": "skipped",
            "alive": false,
            "ready": false,
            "reason": detect.reason,
            "detect": detect
        }));
    }

    let install_command = detect.install_command.clone();
    let dev_command = detect.dev_command.clone().expect("checked above");
    if env_truthy("AXHUB_SCAFFOLD_DEV_DRY_RUN") {
        return Ok(json!({
            "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
            "action": "planned",
            "alive": false,
            "ready": false,
            "install_command": install_command,
            "dev_command": dev_command,
            "reason": "dry_run"
        }));
    }

    if let Some(command) = install_command.as_ref() {
        let status = Command::new(&command[0])
            .args(&command[1..])
            .current_dir(cwd)
            .stdin(Stdio::null())
            .status()
            .with_context(|| format!("spawn dependency install {}", command.join(" ")))?;
        if !status.success() {
            return Ok(json!({
                "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
                "action": "failed",
                "alive": false,
                "ready": false,
                "reason": "install_failed",
                "exit_code": status.code(),
                "install_command": command
            }));
        }
    }

    let axhub_dir = cwd.join(".axhub");
    fs::create_dir_all(&axhub_dir)?;
    let log_path = axhub_dir.join("scaffold-dev.log");
    let stdout = open_log(&log_path)?;
    let stderr = stdout.try_clone()?;
    let mut command = Command::new(&dev_command[0]);
    command
        .args(&dev_command[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    configure_detached_process(&mut command);
    let child = command
        .spawn()
        .with_context(|| format!("spawn dev command {}", dev_command.join(" ")))?;
    let pid = child.id();
    drop(child);

    let (url, alive_after_wait) = wait_for_local_url(pid, &log_path, Duration::from_secs(15));
    if !alive_after_wait {
        return Ok(json!({
            "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
            "action": "failed",
            "alive": false,
            "ready": false,
            "pid": pid,
            "url": Value::Null,
            "port": Value::Null,
            "install_command": install_command,
            "dev_command": dev_command,
            "reason": "dev_process_exited"
        }));
    }

    let port = url.as_deref().and_then(parse_port);
    let now = now_ts();
    let state = ScaffoldDevState {
        schema_version: SCAFFOLD_DEV_SCHEMA_VERSION.to_string(),
        pid,
        url: url.clone(),
        port,
        command: dev_command.clone(),
        log_path: log_path.to_string_lossy().to_string(),
        cwd: cwd.to_string_lossy().to_string(),
        started_at: now.clone(),
        updated_at: now,
    };
    write_scaffold_dev_state(cwd, &state)?;

    Ok(json!({
        "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
        "action": "started",
        "alive": true,
        "ready": state.url.is_some(),
        "pid": pid,
        "url": url,
        "port": port,
        "install_command": install_command,
        "dev_command": dev_command,
        "reason": if state.url.is_some() { "ready" } else { "url_not_detected" }
    }))
}

fn scaffold_dev_status(cwd: &Path) -> anyhow::Result<Value> {
    match read_scaffold_dev_state(cwd)? {
        Some(state) => {
            let health = scaffold_dev_health(cwd, &state);
            Ok(json!({
                "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
                "alive": health.alive,
                "ready": health.ready,
                "pid": state.pid,
                "url": state.url,
                "port": state.port,
                "reason": health.reason
            }))
        }
        None => Ok(json!({
            "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
            "alive": false,
            "ready": false,
            "reason": "state_missing"
        })),
    }
}

fn scaffold_dev_stop(cwd: &Path) -> anyhow::Result<Value> {
    let state = read_scaffold_dev_state(cwd)?;
    let mut stopped = false;
    let mut reason = "state_missing";
    if let Some(state) = state {
        let health = scaffold_dev_health(cwd, &state);
        if health.alive {
            stopped = terminate_pid_group(state.pid);
            reason = if stopped { "stopped" } else { "stop_failed" };
        } else {
            reason = health.reason;
        }
    }
    let _ = fs::remove_file(scaffold_dev_state_path(cwd));
    Ok(json!({
        "schema_version": SCAFFOLD_DEV_SCHEMA_VERSION,
        "stopped": stopped,
        "alive": false,
        "ready": false,
        "reason": reason
    }))
}

struct ScaffoldDevHealth {
    alive: bool,
    ready: bool,
    reason: &'static str,
}

fn scaffold_dev_health(cwd: &Path, state: &ScaffoldDevState) -> ScaffoldDevHealth {
    if timestamp_stale(&state.updated_at) {
        return ScaffoldDevHealth {
            alive: false,
            ready: false,
            reason: "state_stale",
        };
    }
    if state.cwd != cwd.to_string_lossy() {
        return ScaffoldDevHealth {
            alive: false,
            ready: false,
            reason: "state_mismatch",
        };
    }
    if !is_pid_alive(state.pid) {
        return ScaffoldDevHealth {
            alive: false,
            ready: false,
            reason: "stale_pid",
        };
    }
    if !process_matches_command(state.pid, &state.command) {
        return ScaffoldDevHealth {
            alive: false,
            ready: false,
            reason: "identity_mismatch",
        };
    }
    ScaffoldDevHealth {
        alive: true,
        ready: state.url.is_some(),
        reason: if state.url.is_some() {
            "running"
        } else {
            "url_not_detected"
        },
    }
}

fn timestamp_stale(ts: &str) -> bool {
    let Ok(parsed) = DateTime::parse_from_rfc3339(ts) else {
        return true;
    };
    Utc::now()
        .signed_duration_since(parsed.with_timezone(&Utc))
        .num_seconds()
        > SCAFFOLD_DEV_STATE_TTL_SECS
}

fn parse_json_only(command: &str, args: &[String]) -> anyhow::Result<()> {
    for arg in args {
        match arg.as_str() {
            "--json" => {}
            other => anyhow::bail!("axhub-helpers {command}: unknown option \"{other}\""),
        }
    }
    Ok(())
}

fn read_dev_script(cwd: &Path) -> anyhow::Result<Option<String>> {
    let package_json = cwd.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(package_json)?;
    let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    Ok(value
        .get("scripts")
        .and_then(|scripts| scripts.get("dev"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|script| !script.trim().is_empty()))
}

fn node_available() -> bool {
    match std::env::var("AXHUB_SCAFFOLD_NODE_AVAILABLE") {
        Ok(value) if matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO") => {
            return false
        }
        Ok(value) if matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES") => {
            return true
        }
        _ => {}
    }
    Command::new("node")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn package_manager_name(pm: PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm",
        PackageManager::Pnpm => "pnpm",
        PackageManager::Yarn => "yarn",
        PackageManager::Bun => "bun",
    }
}

fn install_command_for(pm: PackageManager) -> Vec<String> {
    match pm {
        PackageManager::Npm => vec!["npm", "install", "--ignore-scripts"],
        PackageManager::Pnpm => vec!["pnpm", "install", "--ignore-scripts"],
        PackageManager::Yarn => vec!["yarn", "install", "--ignore-scripts"],
        PackageManager::Bun => vec!["bun", "install", "--ignore-scripts"],
    }
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn dev_command_for(pm: PackageManager) -> Vec<String> {
    vec![package_manager_name(pm), "run", "dev"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn scaffold_dev_state_path(cwd: &Path) -> PathBuf {
    cwd.join(SCAFFOLD_DEV_STATE_RELATIVE_PATH)
}

fn read_scaffold_dev_state(cwd: &Path) -> anyhow::Result<Option<ScaffoldDevState>> {
    let path = scaffold_dev_state_path(cwd);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    match serde_json::from_str::<ScaffoldDevState>(&raw) {
        Ok(state) if state.schema_version == SCAFFOLD_DEV_SCHEMA_VERSION => Ok(Some(state)),
        _ => {
            let _ = fs::remove_file(path);
            Ok(None)
        }
    }
}

fn write_scaffold_dev_state(cwd: &Path, state: &ScaffoldDevState) -> anyhow::Result<()> {
    let path = scaffold_dev_state_path(cwd);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(state)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn open_log(path: &Path) -> anyhow::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("open scaffold dev log {}", path.display()))
}

#[cfg(unix)]
fn configure_detached_process(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| {
            let _ = libc::setsid();
            Ok(())
        });
    }
}

#[cfg(windows)]
fn configure_detached_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
}

#[cfg(not(any(unix, windows)))]
fn configure_detached_process(_command: &mut Command) {}

fn wait_for_local_url(pid: u32, log_path: &Path, timeout: Duration) -> (Option<String>, bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(mut file) = File::open(log_path) {
            let mut raw = String::new();
            let _ = file.read_to_string(&mut raw);
            if let Some(url) = first_local_url(&raw) {
                return (Some(url), is_pid_alive(pid));
            }
        }
        if !is_pid_alive(pid) {
            return (None, false);
        }
        sleep(Duration::from_millis(250));
    }
    (None, is_pid_alive(pid))
}

fn first_local_url(raw: &str) -> Option<String> {
    let re = Regex::new(r"http://(?:localhost|127\.0\.0\.1|0\.0\.0\.0):[0-9]+").ok()?;
    re.find(raw)
        .map(|m| m.as_str().replace("http://0.0.0.0:", "http://localhost:"))
}

fn parse_port(url: &str) -> Option<u16> {
    url.rsplit(':').next()?.parse().ok()
}

fn command_basename(command: &[String]) -> Option<String> {
    let first = command.first()?;
    Path::new(first)
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .or_else(|| Some(first.to_string()))
}

#[cfg(unix)]
fn process_matches_command(pid: u32, command: &[String]) -> bool {
    let Some(name) = command_basename(command) else {
        return false;
    };
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "args="])
        .output();
    output
        .ok()
        .and_then(|out| out.status.success().then_some(out.stdout))
        .and_then(|stdout| String::from_utf8(stdout).ok())
        .map(|args| args.contains(&name))
        .unwrap_or(false)
}

#[cfg(windows)]
fn process_matches_command(pid: u32, command: &[String]) -> bool {
    let Some(name) = command_basename(command) else {
        return false;
    };
    let output = Command::new("wmic")
        .args([
            "process",
            "where",
            &format!("processid={pid}"),
            "get",
            "CommandLine",
            "/VALUE",
        ])
        .output();
    output
        .ok()
        .and_then(|out| out.status.success().then_some(out.stdout))
        .and_then(|stdout| String::from_utf8(stdout).ok())
        .map(|args| args.contains(&name))
        .unwrap_or_else(|| is_pid_alive(pid))
}

#[cfg(not(any(unix, windows)))]
fn process_matches_command(_pid: u32, _command: &[String]) -> bool {
    false
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    let pid = pid as libc::pid_t;
    if pid <= 0 {
        return false;
    }
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output();
    output
        .map(|out| {
            out.status.success()
                && String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .any(|line| line.contains(&format!("\"{pid}\"")))
        })
        .unwrap_or(false)
}

#[cfg(not(any(unix, windows)))]
fn is_pid_alive(_pid: u32) -> bool {
    false
}

#[cfg(unix)]
fn terminate_pid_group(pid: u32) -> bool {
    let pid = pid as libc::pid_t;
    if pid <= 0 {
        return false;
    }
    let group_result = unsafe { libc::kill(-pid, libc::SIGTERM) };
    if group_result == 0 {
        return true;
    }
    unsafe { libc::kill(pid, libc::SIGTERM) == 0 }
}

#[cfg(windows)]
fn terminate_pid_group(pid: u32) -> bool {
    Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(any(unix, windows)))]
fn terminate_pid_group(_pid: u32) -> bool {
    false
}

fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

//! settings_merge — 7-branch `~/.claude/settings.json` merge foundation.
//!
//! v0.5.13: pure library + manual CLI subcommand only. No auto-trigger.
//! v0.6.0 will call `merge(MergeOptions { silent: true, .. })` from SessionStart.
//!
//! 7-branch decision table:
//!   1/2  file absent or empty/whitespace  → Created
//!   3    valid JSON, statusLine absent    → Merged
//!   4    statusLine.command == default    → NoOp (idempotent)
//!   5    statusLine.command other value   → PreservedOther
//!   6    invalid JSON                     → InvalidJson (never write)
//!   7    statusLine object, missing field → PartialSchema
//!   8    dir/file not writable            → PermissionDenied

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::Serialize;
use serde_json::{json, Value};

const LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_POLL: Duration = Duration::from_millis(100);

/// Options for a single merge call.
#[derive(Debug, Clone)]
pub struct MergeOptions {
    /// false → print Korean systemMessage to stderr.
    /// true  → suppress stderr (v0.6.0 hook caller, JSON-only mode).
    pub silent: bool,
    /// None → default `${CLAUDE_PLUGIN_ROOT}/bin/statusline.{sh,ps1}`.
    /// Some(path) → use given literal (v0.6.0 orphan stub path).
    pub command_path_override: Option<PathBuf>,
    pub scope: Scope,
    /// true → compute outcome, never write. false → atomic write.
    pub dry_run: bool,
}

/// Which settings.json to target.
#[derive(Debug, Clone)]
pub enum Scope {
    User,
    Project,
    Auto,
}

/// 7-branch merge decision outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeOutcome {
    Created,
    Merged,
    NoOp,
    PreservedOther,
    InvalidJson,
    PartialSchema,
    PermissionDenied,
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn user_settings_path() -> PathBuf {
    home_dir().join(".claude").join("settings.json")
}

fn project_settings_path() -> anyhow::Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse --show-toplevel 실패")?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse: git repository 안이에요");
    }
    let toplevel = String::from_utf8(output.stdout)
        .context("git rev-parse: non-UTF-8 output")?;
    Ok(PathBuf::from(toplevel.trim())
        .join(".claude")
        .join("settings.json"))
}

/// Resolve settings.json path from scope.
pub fn settings_path(scope: &Scope) -> anyhow::Result<PathBuf> {
    match scope {
        Scope::User => Ok(user_settings_path()),
        Scope::Project => project_settings_path(),
        Scope::Auto => resolve_auto_scope(),
    }
}

fn resolve_auto_scope() -> anyhow::Result<PathBuf> {
    let plugin_root = std::env::var("CLAUDE_PLUGIN_ROOT")
        .context("CLAUDE_PLUGIN_ROOT 미설정. --scope user 또는 --scope project 로 명시해주세요.")?;
    let plugin_root = PathBuf::from(&plugin_root);

    // Check user scope: ~/.claude/plugins/
    let user_plugins = home_dir().join(".claude").join("plugins");
    if plugin_root.starts_with(&user_plugins) {
        return Ok(user_settings_path());
    }

    // Check project scope: $(git toplevel)/.claude/plugins/
    if let Ok(proj_path) = project_settings_path() {
        if let Some(claude_dir) = proj_path.parent() {
            let proj_plugins = claude_dir.join("plugins");
            if plugin_root.starts_with(&proj_plugins) {
                return Ok(proj_path);
            }
        }
    }

    anyhow::bail!(
        "Auto scope ambiguous: CLAUDE_PLUGIN_ROOT ({}) 가 user plugins ({}) 또는 project plugins 에 속하지 않아요. --scope user 또는 --scope project 로 명시해주세요.",
        plugin_root.display(),
        user_plugins.display()
    )
}

// ---------------------------------------------------------------------------
// Command path
// ---------------------------------------------------------------------------

/// Default command path written to settings.json (unresolved literal).
/// Claude Code expands `${CLAUDE_PLUGIN_ROOT}` at runtime.
pub fn default_command_path(override_path: Option<&Path>) -> String {
    if let Some(p) = override_path {
        return p.to_string_lossy().into_owned();
    }
    if cfg!(target_os = "windows") {
        "${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1".to_string()
    } else {
        "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh".to_string()
    }
}

// ---------------------------------------------------------------------------
// Atomic write + locking
// ---------------------------------------------------------------------------

/// Print Korean systemMessage to stderr unless silent.
fn emit(silent: bool, msg: &str) {
    if !silent {
        eprintln!("{msg}");
    }
}

/// Acquire exclusive fslock on `<settings_path>.lock` with 5s timeout.
fn acquire_lock(lock_path: &Path) -> anyhow::Result<fslock::LockFile> {
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("디렉토리 생성 실패: {}", parent.display()))?;
        }
    }
    let mut lock = fslock::LockFile::open(lock_path)
        .with_context(|| format!("lock file 열기 실패: {}", lock_path.display()))?;
    let start = Instant::now();
    loop {
        if lock.try_lock()? {
            return Ok(lock);
        }
        if start.elapsed() >= LOCK_TIMEOUT {
            anyhow::bail!(
                "axhub: settings.json lock 획득 실패. 다른 process 가 잡고 있어요. 잠시 후 재시도해주세요."
            );
        }
        std::thread::sleep(LOCK_POLL);
    }
}

/// Atomic write: `.tmp` sibling in same dir → fsync → rename.
/// Same filesystem guaranteed (no cross-device rename).
fn atomic_write(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    fs::create_dir_all(parent)
        .with_context(|| format!("디렉토리 생성 실패: {}", parent.display()))?;

    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));

    let mut opts = fs::OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut f = opts
        .open(&tmp)
        .with_context(|| format!("temp file 쓰기 실패: {}", tmp.display()))?;
    f.write_all(data)?;
    f.sync_all()?;
    drop(f);

    fs::rename(&tmp, path).with_context(|| {
        format!(
            "atomic rename 실패: {} → {}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
}

/// Write one-generation .bak atomically before mutating settings.json.
fn write_bak(settings_path: &Path, content: &[u8]) -> anyhow::Result<()> {
    let bak = {
        let mut p = settings_path.to_path_buf();
        let name = format!(
            "{}.bak",
            p.file_name().unwrap_or_default().to_string_lossy()
        );
        p.set_file_name(name);
        p
    };
    atomic_write(&bak, content)
}

/// Check if writing to the parent directory of `path` is likely to succeed.
fn check_writable(path: &Path) -> bool {
    let parent = match path.parent().filter(|p| !p.as_os_str().is_empty()) {
        Some(p) => p,
        None => return true,
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(parent) {
            // Owner write bit.
            return meta.permissions().mode() & 0o200 != 0;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Core merge logic
// ---------------------------------------------------------------------------

/// Build the statusLine JSON value.
fn statusline_value(command: &str) -> Value {
    json!({
        "type": "command",
        "command": command,
        "padding": 0
    })
}

/// Build a fresh settings.json containing only statusLine.
fn new_settings_json(command: &str) -> String {
    let v = json!({
        "statusLine": {
            "type": "command",
            "command": command,
            "padding": 0
        }
    });
    serde_json::to_string_pretty(&v).expect("serde_json serialization never fails for static json")
}

/// Execute the 7-branch merge decision.
///
/// Lock is held from before read-parse until after rename (TOCTOU-safe).
pub fn merge(opts: MergeOptions) -> anyhow::Result<MergeOutcome> {
    let path = settings_path(&opts.scope)?;
    let command = default_command_path(opts.command_path_override.as_deref());

    // Lock file lives next to settings.json.
    let lock_path = {
        let name = format!(
            "{}.lock",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        path.with_file_name(name)
    };

    // Acquire lock BEFORE read-parse (TOCTOU guard).
    let _lock_guard = acquire_lock(&lock_path)?;

    // Read existing content. Absent = treat as empty.
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(_) => {
            emit(
                opts.silent,
                "axhub: ~/.claude/ 디렉토리 쓰기 권한 없어요. chmod u+w ~/.claude 후 재시도해주세요.",
            );
            return Ok(MergeOutcome::PermissionDenied);
        }
    };

    // Branch 1 / 2: file absent or empty/whitespace.
    if raw.trim().is_empty() {
        if opts.dry_run {
            emit(opts.silent, "axhub: settings.json 만들고 statusLine 추가할 예정이에요. (dry-run)");
            return Ok(MergeOutcome::Created);
        }
        if !check_writable(&path) {
            emit(
                opts.silent,
                "axhub: ~/.claude/ 디렉토리 쓰기 권한 없어요. chmod u+w ~/.claude 후 재시도해주세요.",
            );
            return Ok(MergeOutcome::PermissionDenied);
        }
        // No existing content → no .bak needed.
        if let Err(e) = atomic_write(&path, new_settings_json(&command).as_bytes()) {
            emit(
                opts.silent,
                "axhub: ~/.claude/ 디렉토리 쓰기 권한 없어요. chmod u+w ~/.claude 후 재시도해주세요.",
            );
            return Err(e);
        }
        emit(opts.silent, "axhub: settings.json 만들고 statusLine 추가했어요.");
        return Ok(MergeOutcome::Created);
    }

    // Branch 6: invalid JSON — never write.
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            emit(
                opts.silent,
                "axhub: settings.json JSON syntax error. 파싱 안 돼서 자동 작업 건너뛰었어요. 직접 수정 후 /axhub:doctor 실행해주세요.",
            );
            return Ok(MergeOutcome::InvalidJson);
        }
    };

    // Must be a JSON object.
    let mut obj = match parsed {
        Value::Object(m) => m,
        _ => {
            emit(
                opts.silent,
                "axhub: settings.json JSON syntax error. 파싱 안 돼서 자동 작업 건너뛰었어요. 직접 수정 후 /axhub:doctor 실행해주세요.",
            );
            return Ok(MergeOutcome::InvalidJson);
        }
    };

    // Branch 3: statusLine key absent → shallow-merge add.
    if !obj.contains_key("statusLine") {
        if opts.dry_run {
            emit(opts.silent, "axhub: settings.json 에 statusLine 추가할 예정이에요. (dry-run)");
            return Ok(MergeOutcome::Merged);
        }
        if !check_writable(&path) {
            emit(
                opts.silent,
                "axhub: ~/.claude/ 디렉토리 쓰기 권한 없어요. chmod u+w ~/.claude 후 재시도해주세요.",
            );
            return Ok(MergeOutcome::PermissionDenied);
        }
        // Write .bak first (atomic), then mutate.
        write_bak(&path, raw.as_bytes())?;
        obj.insert("statusLine".to_string(), statusline_value(&command));
        let new_content = serde_json::to_string_pretty(&Value::Object(obj))?;
        if let Err(e) = atomic_write(&path, new_content.as_bytes()) {
            // Best-effort restore: .bak → settings.json.
            let bak = path.with_extension("json.bak");
            let _ = fs::rename(&bak, &path);
            emit(
                opts.silent,
                "axhub: ~/.claude/ 디렉토리 쓰기 권한 없어요. chmod u+w ~/.claude 후 재시도해주세요.",
            );
            return Err(e);
        }
        emit(
            opts.silent,
            "axhub: settings.json 에 statusLine 추가했어요. Claude Code 재시작해주세요.",
        );
        return Ok(MergeOutcome::Merged);
    }

    // statusLine key is present — inspect it.
    let status_line = &obj["statusLine"];

    match status_line {
        Value::Object(sl_obj) => {
            let stored_type = sl_obj.get("type").and_then(Value::as_str);
            let stored_cmd = sl_obj.get("command").and_then(Value::as_str);

            // Branch 7: partial schema (missing type or command).
            if stored_type.is_none() || stored_cmd.is_none() {
                emit(
                    opts.silent,
                    "axhub: settings.json 의 statusLine 이 incomplete 해요. 자동 변경 안 했어요. /axhub:enable-statusline 으로 수동 결정해주세요.",
                );
                return Ok(MergeOutcome::PartialSchema);
            }

            let stored_cmd = stored_cmd.unwrap();

            // Branch 4: command matches default literal → no-op.
            if stored_cmd == command {
                return Ok(MergeOutcome::NoOp);
            }

            // Branch 5: different command → preserve user setting, warn.
            emit(
                opts.silent,
                &format!(
                    "axhub: settings.json 에 다른 statusLine 이 있어요 ('{stored_cmd}'). 자동 변경 안 했어요. /axhub:enable-statusline 으로 수동 결정해주세요."
                ),
            );
            Ok(MergeOutcome::PreservedOther)
        }
        _ => {
            // statusLine is not an object (string, number, etc.) → Branch 7 style.
            emit(
                opts.silent,
                "axhub: settings.json 의 statusLine 이 incomplete 해요. 자동 변경 안 했어요. /axhub:enable-statusline 으로 수동 결정해주세요.",
            );
            Ok(MergeOutcome::PartialSchema)
        }
    }
}

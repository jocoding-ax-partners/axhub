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

use crate::orphan_stub;

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
    let toplevel = String::from_utf8(output.stdout).context("git rev-parse: non-UTF-8 output")?;
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

/// Default command path written to settings.json.
///
/// When `override_path = None`, returns the orphan stub absolute path
/// (plugin-context-independent, survives plugin uninstall).
/// Falls back to the legacy `${CLAUDE_PLUGIN_ROOT}` literal only when
/// the state dir is unavailable (XDG_STATE_HOME / HOME unset).
///
/// When `override_path = Some(p)`, emits a deprecation warning and returns
/// the literal path for backward compatibility (removed in v0.7.0).
pub fn default_command_path(override_path: Option<&Path>) -> String {
    let script_path = if let Some(p) = override_path {
        eprintln!(
            "axhub: command_path_override 는 deprecated 예요. v0.7.0 에서 제거될 예정이에요. orphan stub path 를 사용해주세요."
        );
        p.to_string_lossy().into_owned()
    } else if let Some(p) = orphan_stub::stub_path() {
        p.to_string_lossy().into_owned()
    } else if cfg!(target_os = "windows") {
        "${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1".to_string()
    } else {
        "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh".to_string()
    };
    command_for_platform_script(&script_path, cfg!(target_os = "windows"))
}

/// Convert a script path into the `statusLine.command` string for a platform.
///
/// Windows must not store a bare `.ps1` path: stock Win10/11 commonly has
/// `ExecutionPolicy=Restricted`, and `cmd` does not execute `.ps1` via PATHEXT.
/// Keep the explicit PowerShell bypass form used by the public snippet.
pub fn command_for_platform_script(script_path: &str, is_windows: bool) -> String {
    if is_windows {
        format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
            script_path
        )
    } else {
        script_path.to_string()
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
        path.file_name().unwrap_or_default().to_string_lossy()
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

    fs::rename(&tmp, path)
        .with_context(|| format!("atomic rename 실패: {} → {}", tmp.display(), path.display()))?;
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
            emit(
                opts.silent,
                "axhub: settings.json 만들고 statusLine 추가할 예정이에요. (dry-run)",
            );
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
        emit(
            opts.silent,
            "axhub: settings.json 만들고 statusLine 추가했어요.",
        );
        return Ok(MergeOutcome::Created);
    }

    // Branch 6: invalid JSON — never write.
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            emit(
                opts.silent,
                "axhub: settings.json JSON syntax error. 파싱 안 돼서 자동 작업 건너뛰었어요. 직접 수정 후 \"설치 상태 확인해줘\"라고 말해 주세요.",
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
                "axhub: settings.json JSON syntax error. 파싱 안 돼서 자동 작업 건너뛰었어요. 직접 수정 후 \"설치 상태 확인해줘\"라고 말해 주세요.",
            );
            return Ok(MergeOutcome::InvalidJson);
        }
    };

    // Branch 3: statusLine key absent → shallow-merge add.
    if !obj.contains_key("statusLine") {
        if opts.dry_run {
            emit(
                opts.silent,
                "axhub: settings.json 에 statusLine 추가할 예정이에요. (dry-run)",
            );
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
                    "axhub: settings.json 의 statusLine 이 incomplete 해요. 자동 변경 안 했어요. \"statusLine 설정 도와줘\"라고 말해 수동으로 결정해 주세요.",
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
                    "axhub: settings.json 에 다른 statusLine 이 있어요 ('{stored_cmd}'). 자동 변경 안 했어요. \"statusLine 설정 도와줘\"라고 말해 수동으로 결정해 주세요."
                ),
            );
            Ok(MergeOutcome::PreservedOther)
        }
        _ => {
            // statusLine is not an object (string, number, etc.) → Branch 7 style.
            emit(
                opts.silent,
                "axhub: settings.json 의 statusLine 이 incomplete 해요. 자동 변경 안 했어요. \"statusLine 설정 도와줘\"라고 말해 수동으로 결정해 주세요.",
            );
            Ok(MergeOutcome::PartialSchema)
        }
    }
}

// ---------------------------------------------------------------------------
// Migration — stale ${CLAUDE_PLUGIN_ROOT} literal → orphan stub absolute path
// ---------------------------------------------------------------------------

/// Stale substring that identifies a settings.json written by the old code path.
const STALE_SUBSTRING: &str = "${CLAUDE_PLUGIN_ROOT}/bin/statusline.";

/// Per-path migration outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrateOutcome {
    /// Stale command replaced with orphan stub absolute path.
    Migrated,
    /// Already points at stub absolute path — no change needed.
    NoOp,
    /// File exists but contains no stale literal — nothing to migrate.
    NoStaleFound,
    /// Project scope + git-tracked → WARN-ONLY, no write.
    WarnGitTracked,
    /// Invalid JSON — never write.
    InvalidJson,
    /// Directory or file not writable.
    PermissionDenied,
    /// settings.json does not exist at this scope.
    NotFound,
    /// Stale literal detected but dry-run mode — no write.
    DryRun,
    /// Stub absolute path unavailable (state dir unresolvable) — cannot migrate.
    StubUnavailable,
}

/// Detect and rewrite stale `${CLAUDE_PLUGIN_ROOT}` literals in settings.json.
///
/// When `scope = Auto`, both user and project settings files are scanned.
/// Returns a vec of `(scope_label, outcome)` pairs.
pub fn migrate_stale_command_path(
    scope: &Scope,
    dry_run: bool,
) -> anyhow::Result<Vec<(String, MigrateOutcome)>> {
    let stub = orphan_stub::stub_path();

    let targets: Vec<(String, anyhow::Result<PathBuf>)> = match scope {
        Scope::User => vec![("user".to_string(), Ok(user_settings_path()))],
        Scope::Project => vec![("project".to_string(), project_settings_path())],
        Scope::Auto => {
            let mut v: Vec<(String, anyhow::Result<PathBuf>)> =
                vec![("user".to_string(), Ok(user_settings_path()))];
            // Project scope is best-effort — skip when not in a git repo.
            if let Ok(p) = project_settings_path() {
                v.push(("project".to_string(), Ok(p)));
            }
            v
        }
    };

    let mut results = Vec::new();
    for (label, path_res) in targets {
        let path = match path_res {
            Ok(p) => p,
            Err(_) => {
                results.push((label, MigrateOutcome::NotFound));
                continue;
            }
        };
        let outcome = migrate_single_path(&path, &label, &stub, dry_run)?;
        results.push((label, outcome));
    }
    Ok(results)
}

fn migrate_single_path(
    path: &Path,
    scope_label: &str,
    stub: &Option<PathBuf>,
    dry_run: bool,
) -> anyhow::Result<MigrateOutcome> {
    if !path.exists() {
        return Ok(MigrateOutcome::NotFound);
    }

    let lock_path = {
        let name = format!(
            "{}.lock",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        path.with_file_name(name)
    };
    let _lock = acquire_lock(&lock_path)?;

    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(MigrateOutcome::NotFound),
        Err(_) => return Ok(MigrateOutcome::PermissionDenied),
    };

    // Branch 6: invalid JSON — never write.
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Ok(MigrateOutcome::InvalidJson),
    };
    let mut obj = match parsed {
        Value::Object(m) => m,
        _ => return Ok(MigrateOutcome::InvalidJson),
    };

    // Detect stale literal.
    let current_cmd = obj
        .get("statusLine")
        .and_then(|sl| sl.get("command"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    let is_stale = current_cmd
        .as_deref()
        .map(|cmd| cmd.contains(STALE_SUBSTRING))
        .unwrap_or(false);

    if !is_stale {
        // Check if already migrated to stub absolute path (idempotent guard).
        if let (Some(cmd), Some(stub_p)) = (current_cmd.as_deref(), stub.as_ref()) {
            let expected =
                command_for_platform_script(&stub_p.to_string_lossy(), cfg!(target_os = "windows"));
            if cmd == expected {
                return Ok(MigrateOutcome::NoOp);
            }
        }
        return Ok(MigrateOutcome::NoStaleFound);
    }

    // Git-tracked detect (S7): project scope only — WARN-ONLY, no write.
    if scope_label == "project" && is_git_tracked(path) {
        eprintln!(
            "axhub: {} 는 git 에 추적 중이에요. 자동 수정 안 했어요. 직접 편집 후 commit 해주세요.",
            path.display()
        );
        return Ok(MigrateOutcome::WarnGitTracked);
    }

    if dry_run {
        eprintln!(
            "axhub: {} — stale statusLine.command 감지했어요. (dry-run, 변경 없음)",
            path.display()
        );
        return Ok(MigrateOutcome::DryRun);
    }

    // Compute new command from stub absolute path.
    let stub_p = match stub.as_ref() {
        Some(p) => p,
        None => {
            eprintln!("axhub: orphan stub path 를 확인할 수 없어요. migrate 를 건너뛰었어요.");
            return Ok(MigrateOutcome::StubUnavailable);
        }
    };
    let new_cmd =
        command_for_platform_script(&stub_p.to_string_lossy(), cfg!(target_os = "windows"));

    // Atomic rewrite: .bak → mutate → rename.
    write_bak(path, raw.as_bytes())?;

    if let Some(sl) = obj.get_mut("statusLine").and_then(Value::as_object_mut) {
        sl.insert("command".to_string(), Value::String(new_cmd));
    }

    let new_content = serde_json::to_string_pretty(&Value::Object(obj))?;
    atomic_write(path, new_content.as_bytes())?;

    eprintln!(
        "axhub: {} statusLine.command 를 orphan stub path 로 마이그레이션했어요.",
        path.display()
    );
    Ok(MigrateOutcome::Migrated)
}

/// Check whether `path` is tracked by git (`git ls-files --error-unmatch`).
fn is_git_tracked(path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch", &path.to_string_lossy()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Guard: 테스트 동안 HOME + XDG_STATE_HOME + CLAUDE_PLUGIN_ROOT 격리.
    /// Drop 시 원래 값 복원. PROCESS_ENV_LOCK 으로 크로스-테스트 env 경쟁 방지.
    struct EnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        _xdg_dir: tempfile::TempDir,
        _home_dir: tempfile::TempDir,
        old_xdg: Option<std::ffi::OsString>,
        old_home: Option<std::ffi::OsString>,
        old_plugin_root: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn new() -> Self {
            let lock = crate::PROCESS_ENV_LOCK
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            let xdg_dir = tempfile::tempdir().unwrap();
            let home_dir = tempfile::tempdir().unwrap();
            let old_xdg = std::env::var_os("XDG_STATE_HOME");
            let old_home = std::env::var_os("HOME");
            let old_plugin_root = std::env::var_os("CLAUDE_PLUGIN_ROOT");
            unsafe {
                std::env::set_var("XDG_STATE_HOME", xdg_dir.path());
                std::env::set_var("HOME", home_dir.path());
                std::env::remove_var("CLAUDE_PLUGIN_ROOT");
            }
            Self {
                _lock: lock,
                _xdg_dir: xdg_dir,
                _home_dir: home_dir,
                old_xdg,
                old_home,
                old_plugin_root,
            }
        }

        fn home(&self) -> &std::path::Path {
            self._home_dir.path()
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.old_xdg.take() {
                Some(v) => unsafe { std::env::set_var("XDG_STATE_HOME", v) },
                None => unsafe { std::env::remove_var("XDG_STATE_HOME") },
            }
            match self.old_home.take() {
                Some(v) => unsafe { std::env::set_var("HOME", v) },
                None => unsafe { std::env::remove_var("HOME") },
            }
            match self.old_plugin_root.take() {
                Some(v) => unsafe { std::env::set_var("CLAUDE_PLUGIN_ROOT", v) },
                None => unsafe { std::env::remove_var("CLAUDE_PLUGIN_ROOT") },
            }
        }
    }

    fn write_settings(home: &std::path::Path, content: &str) {
        let claude_dir = home.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("settings.json"), content).unwrap();
    }

    fn read_settings(home: &std::path::Path) -> String {
        fs::read_to_string(home.join(".claude").join("settings.json")).unwrap()
    }

    // ── AC #1: default_command_path(None) returns orphan stub absolute path ───

    #[test]
    fn default_command_path_none_returns_stub_absolute_path() {
        let guard = EnvGuard::new();
        let result = default_command_path(None);
        assert!(
            !result.contains("${CLAUDE_PLUGIN_ROOT}"),
            "must not contain CLAUDE_PLUGIN_ROOT template: {result}"
        );
        assert!(
            result.contains("orphan-stub-statusline"),
            "must reference orphan stub: {result}"
        );
        #[cfg(unix)]
        assert!(
            result.starts_with('/'),
            "must be absolute path on Unix: {result}"
        );
        let _ = guard;
    }

    // ── AC #2: default_command_path(Some(p)) emits deprecation + returns p ───

    #[test]
    fn default_command_path_some_returns_override_path() {
        let guard = EnvGuard::new();
        let override_path = std::path::Path::new("/my/custom/statusline.sh");
        let result = default_command_path(Some(override_path));
        assert!(
            result.contains("/my/custom/statusline.sh"),
            "must contain override path: {result}"
        );
        let _ = guard;
    }

    // ── AC #18: CLAUDE_PLUGIN_ROOT=OMC path — result independent ──────────────

    #[test]
    fn default_command_path_independent_of_claude_plugin_root() {
        let guard = EnvGuard::new();
        unsafe {
            std::env::set_var(
                "CLAUDE_PLUGIN_ROOT",
                "/tmp/omc-fake-root/oh-my-claudecode/4.13.7",
            );
        }
        let result = default_command_path(None);
        assert!(
            !result.contains("omc-fake-root"),
            "must not contain OMC plugin root: {result}"
        );
        assert!(
            !result.contains("oh-my-claudecode"),
            "must not contain OMC plugin name: {result}"
        );
        assert!(
            !result.contains("${CLAUDE_PLUGIN_ROOT}"),
            "must not contain unresolved template: {result}"
        );
        assert!(
            result.contains("orphan-stub-statusline"),
            "must reference orphan stub: {result}"
        );
        let _ = guard;
    }

    // ── AC #7: Windows quoting — paths with spaces ────────────────────────────

    #[test]
    fn command_for_platform_script_quotes_paths_with_spaces_on_windows() {
        let path_with_spaces = "/path with spaces/stub.ps1";
        let result = command_for_platform_script(path_with_spaces, true);
        assert!(
            result.contains("powershell.exe"),
            "Windows command must use powershell.exe: {result}"
        );
        assert!(
            result.contains('"'),
            "Windows command must quote the path: {result}"
        );
        assert!(
            result.contains(path_with_spaces),
            "Windows command must contain original path: {result}"
        );
        let unix = command_for_platform_script(path_with_spaces, false);
        assert_eq!(unix, path_with_spaces, "Unix must return path as-is");
    }

    // ── AC #8: migrate detects stale literal + atomic write ──────────────────

    #[test]
    fn migrate_stale_command_path_replaces_stale_literal() {
        let guard = EnvGuard::new();
        let stale_cmd = "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh";
        write_settings(
            guard.home(),
            &serde_json::to_string(&serde_json::json!({
                "statusLine": { "type": "command", "command": stale_cmd, "padding": 0 }
            }))
            .unwrap(),
        );

        let results = migrate_stale_command_path(&Scope::User, false).unwrap();
        let (_, outcome) = results.iter().find(|(l, _)| l == "user").unwrap();
        assert_eq!(*outcome, MigrateOutcome::Migrated, "must be Migrated");

        let new_content: serde_json::Value =
            serde_json::from_str(&read_settings(guard.home())).unwrap();
        let new_cmd = new_content["statusLine"]["command"].as_str().unwrap();
        assert!(
            !new_cmd.contains("${CLAUDE_PLUGIN_ROOT}"),
            "migrated command must not contain stale literal: {new_cmd}"
        );
        assert!(
            new_cmd.contains("orphan-stub-statusline"),
            "migrated command must reference stub: {new_cmd}"
        );
        // .bak must exist (atomic write committed)
        assert!(
            guard
                .home()
                .join(".claude")
                .join("settings.json.bak")
                .exists(),
            ".bak must exist after migration"
        );
        let _ = guard;
    }

    // ── AC #9: migrate already-stub path → NoOp ──────────────────────────────

    #[test]
    fn migrate_stale_command_path_already_stub_is_noop() {
        let guard = EnvGuard::new();
        let stub_p = crate::orphan_stub::stub_path().expect("stub_path must resolve");
        let stub_cmd =
            command_for_platform_script(&stub_p.to_string_lossy(), cfg!(target_os = "windows"));
        write_settings(
            guard.home(),
            &serde_json::to_string(&serde_json::json!({
                "statusLine": { "type": "command", "command": stub_cmd, "padding": 0 }
            }))
            .unwrap(),
        );
        let original = read_settings(guard.home());

        let results = migrate_stale_command_path(&Scope::User, false).unwrap();
        let (_, outcome) = results.iter().find(|(l, _)| l == "user").unwrap();
        assert_eq!(*outcome, MigrateOutcome::NoOp, "already-stub must be NoOp");
        assert_eq!(
            read_settings(guard.home()),
            original,
            "settings.json must be unchanged for NoOp"
        );
        let _ = guard;
    }

    // ── AC #10: migrate invalid JSON → abort, file unchanged ─────────────────

    #[test]
    fn migrate_stale_command_path_invalid_json_abort() {
        let guard = EnvGuard::new();
        let broken = "{broken json: this is not valid";
        write_settings(guard.home(), broken);

        let results = migrate_stale_command_path(&Scope::User, false).unwrap();
        let (_, outcome) = results.iter().find(|(l, _)| l == "user").unwrap();
        assert_eq!(*outcome, MigrateOutcome::InvalidJson, "must be InvalidJson");
        assert_eq!(
            read_settings(guard.home()),
            broken,
            "settings.json must not be modified for invalid JSON"
        );
        let _ = guard;
    }
}

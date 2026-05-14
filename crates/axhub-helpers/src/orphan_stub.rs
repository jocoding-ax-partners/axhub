//! v0.6.0 — Orphan stub installer.
//!
//! `axhub-helpers orphan-stub --install` writes a thin shell/PowerShell script
//! to `$XDG_STATE_HOME/axhub-plugin/orphan-stub-statusline.{sh,ps1}`.
//!
//! The stub path is what settings.json `statusLine.command` points to — it
//! survives plugin uninstall (user-global state dir, not plugin root).
//!
//! Stub behaviour:
//!   - Plugin present → `exec ${CLAUDE_PLUGIN_ROOT}/bin/statusline.{sh,ps1}`
//!   - Plugin missing → graceful `exit 0` (empty output, no error)

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::runtime_paths::state_dir;

// ---------------------------------------------------------------------------
// Stub script templates
// ---------------------------------------------------------------------------

const STUB_SH: &str = r#"#!/bin/sh
# axhub orphan stub — survives plugin uninstall
# Delegates to the real statusline script if the plugin is still installed.
if [ -x "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh" ]; then
  exec "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh" "$@"
fi
# Plugin removed — graceful empty output (Claude Code shows blank statusline).
exit 0
"#;

const STUB_PS1: &str = r#"# axhub orphan stub (PowerShell) — survives plugin uninstall
# Delegates to the real statusline script if the plugin is still installed.
$statusline = "$env:CLAUDE_PLUGIN_ROOT\bin\statusline.ps1"
if (Test-Path $statusline) {
    & $statusline @args
    exit $LASTEXITCODE
}
# Plugin removed — graceful empty output (Claude Code shows blank statusline).
exit 0
"#;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Resolved orphan stub paths (platform-appropriate).
#[derive(Debug, Clone)]
pub struct StubPaths {
    /// POSIX shell stub path.
    pub sh: PathBuf,
    /// PowerShell stub path (Windows / cross-platform).
    pub ps1: PathBuf,
}

impl StubPaths {
    /// Resolve stub paths from the canonical state dir.
    pub fn resolve() -> Option<Self> {
        let sd = state_dir()?;
        Some(Self {
            sh: sd.join("orphan-stub-statusline.sh"),
            ps1: sd.join("orphan-stub-statusline.ps1"),
        })
    }

    /// Return the platform-default stub path used in settings.json.
    ///
    /// On Windows returns `ps1`, on all other platforms returns `sh`.
    #[allow(dead_code)]
    pub fn default_path(&self) -> &Path {
        if cfg!(target_os = "windows") {
            &self.ps1
        } else {
            &self.sh
        }
    }
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

/// Write orphan stub scripts to the user-global state dir.
///
/// Returns the platform-default stub path on success.
/// Atomic write (`.tmp` + rename) + `chmod +x` on Unix.
/// Idempotent — overwrites existing stub so it stays up-to-date.
pub fn install() -> anyhow::Result<PathBuf> {
    let paths = StubPaths::resolve()
        .context("state_dir() 를 확인할 수 없어요. XDG_STATE_HOME 또는 HOME 을 설정해주세요.")?;

    let sd = state_dir().unwrap(); // already resolved above
    fs::create_dir_all(&sd).with_context(|| format!("state dir 생성 실패: {}", sd.display()))?;

    atomic_write_stub(&paths.sh, STUB_SH.as_bytes())?;
    set_executable(&paths.sh)?;

    atomic_write_stub(&paths.ps1, STUB_PS1.as_bytes())?;
    // PowerShell scripts don't need +x on Unix, but set it for consistency
    // on systems that might exec them via shebang-aware launchers.
    set_executable(&paths.ps1)?;

    Ok(if cfg!(target_os = "windows") {
        paths.ps1
    } else {
        paths.sh
    })
}

/// Verify that the stub at `path` exists, is readable, and is executable.
///
/// Returns `true` when the stub is ready to be pointed at from settings.json.
pub fn verify(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    is_executable(path)
}

/// Install (if absent) then verify the platform stub.
/// Returns the verified stub path, or `None` if verification fails.
pub fn install_and_verify() -> Option<PathBuf> {
    let path = install().ok()?;
    if verify(&path) {
        Some(path)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn atomic_write_stub(path: &Path, content: &[u8]) -> anyhow::Result<()> {
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

    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o755); // executable from the start
    }
    let mut f = opts
        .open(&tmp)
        .with_context(|| format!("임시 파일 쓰기 실패: {}", tmp.display()))?;
    f.write_all(content)?;
    f.sync_all()?;
    drop(f);

    fs::rename(&tmp, path)
        .with_context(|| format!("atomic rename 실패: {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

fn set_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .with_context(|| format!("metadata 읽기 실패: {}", path.display()))?
            .permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(path, perms)
            .with_context(|| format!("chmod 실패: {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path; // no-op on Windows
    }
    Ok(())
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.exists()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct StateGuard {
        _dir: tempfile::TempDir,
        old_xdg: Option<std::ffi::OsString>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl StateGuard {
        fn new() -> Self {
            let lock = crate::PROCESS_ENV_LOCK
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            let dir = tempfile::tempdir().unwrap();
            let old_xdg = std::env::var_os("XDG_STATE_HOME");
            unsafe { std::env::set_var("XDG_STATE_HOME", dir.path()) }
            Self {
                _dir: dir,
                old_xdg,
                _lock: lock,
            }
        }
    }

    impl Drop for StateGuard {
        fn drop(&mut self) {
            match self.old_xdg.take() {
                Some(v) => unsafe { std::env::set_var("XDG_STATE_HOME", v) },
                None => unsafe { std::env::remove_var("XDG_STATE_HOME") },
            }
        }
    }

    #[test]
    fn install_creates_sh_and_ps1() {
        let _guard = StateGuard::new();
        let path = install().expect("install should succeed");
        assert!(path.exists(), "default stub path should exist");
        let paths = StubPaths::resolve().unwrap();
        assert!(paths.sh.exists(), "sh stub should exist");
        assert!(paths.ps1.exists(), "ps1 stub should exist");
    }

    #[test]
    fn install_is_idempotent() {
        let _guard = StateGuard::new();
        install().unwrap();
        install().unwrap(); // second call must not error
        let paths = StubPaths::resolve().unwrap();
        assert!(paths.sh.exists());
    }

    #[test]
    fn stub_sh_contains_delegate_logic() {
        let _guard = StateGuard::new();
        install().unwrap();
        let paths = StubPaths::resolve().unwrap();
        let content = fs::read_to_string(&paths.sh).unwrap();
        assert!(
            content.contains("CLAUDE_PLUGIN_ROOT"),
            "sh stub should reference CLAUDE_PLUGIN_ROOT"
        );
        assert!(
            content.contains("statusline.sh"),
            "sh stub should delegate to statusline.sh"
        );
    }

    #[test]
    fn stub_ps1_contains_delegate_logic() {
        let _guard = StateGuard::new();
        install().unwrap();
        let paths = StubPaths::resolve().unwrap();
        let content = fs::read_to_string(&paths.ps1).unwrap();
        assert!(
            content.contains("CLAUDE_PLUGIN_ROOT"),
            "ps1 stub should reference CLAUDE_PLUGIN_ROOT"
        );
        assert!(
            content.contains("statusline.ps1"),
            "ps1 stub should delegate to statusline.ps1"
        );
    }

    #[cfg(unix)]
    #[test]
    fn install_sh_is_executable() {
        let _guard = StateGuard::new();
        install().unwrap();
        let paths = StubPaths::resolve().unwrap();
        assert!(is_executable(&paths.sh), "sh stub should be executable");
    }

    #[test]
    fn verify_returns_false_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nonexistent.sh");
        assert!(!verify(&missing));
    }

    #[test]
    fn install_and_verify_returns_some() {
        let _guard = StateGuard::new();
        let result = install_and_verify();
        assert!(result.is_some(), "install_and_verify should succeed");
    }
}

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use serde::Deserialize;
use serde::Serialize;

use crate::preflight::{install_dir_candidates, AXHUB_BIN_NAME};

#[derive(Debug, Clone, Serialize)]
pub struct RepairPathReport {
    pub repaired: bool,
    pub already_present: bool,
    pub disabled: bool,
    pub install_dir: Option<PathBuf>,
    pub shell_rc: Option<PathBuf>,
    pub backup_path: Option<PathBuf>,
    pub message: String,
    pub error: Option<String>,
}

impl RepairPathReport {
    fn disabled() -> Self {
        Self {
            repaired: false,
            already_present: false,
            disabled: true,
            install_dir: None,
            shell_rc: None,
            backup_path: None,
            message: "PATH repair is disabled by AXHUB_DISABLE_PATH_REPAIR".into(),
            error: None,
        }
    }

    fn no_install_dir(message: String) -> Self {
        Self {
            repaired: false,
            already_present: false,
            disabled: false,
            install_dir: None,
            shell_rc: None,
            backup_path: None,
            message,
            error: None,
        }
    }

    fn error(
        install_dir: Option<PathBuf>,
        shell_rc: Option<PathBuf>,
        error: anyhow::Error,
    ) -> Self {
        Self {
            repaired: false,
            already_present: false,
            disabled: false,
            install_dir,
            shell_rc,
            backup_path: None,
            message: "PATH repair could not be completed".into(),
            error: Some(error.to_string()),
        }
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}

fn path_contains_dir(path_value: &str, install_dir: &Path) -> bool {
    std::env::split_paths(path_value).any(|dir| dir == install_dir)
}

fn add_install_dir_to_current_path(install_dir: &Path) {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let current_string = current.to_string_lossy();
    if path_contains_dir(&current_string, install_dir) {
        return;
    }
    let mut parts: Vec<PathBuf> = std::env::split_paths(&current).collect();
    parts.push(install_dir.to_path_buf());
    if let Ok(joined) = std::env::join_paths(parts) {
        std::env::set_var("PATH", joined);
    }
}

pub fn find_installed_dir(explicit_dir: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(dir) = explicit_dir {
        return Some(dir);
    }
    install_dir_candidates()
        .into_iter()
        .find(|dir| dir.join(AXHUB_BIN_NAME).is_file())
}

#[cfg(unix)]
pub fn unix_path_line(install_dir: &Path, home: &Path) -> String {
    if install_dir == home.join(".axhub/bin") {
        r#"export PATH="$HOME/.axhub/bin:$PATH""#.to_string()
    } else {
        format!(
            "export PATH=\"{}:$PATH\"",
            install_dir.to_string_lossy().replace('"', "\\\"")
        )
    }
}

#[cfg(unix)]
fn default_unix_shell_rc(home: &Path) -> Option<PathBuf> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.contains("zsh") {
        Some(home.join(".zshrc"))
    } else {
        if shell.contains("bash") {
            return Some(home.join(".bashrc"));
        }
        if shell.is_empty() {
            #[cfg(target_os = "macos")]
            {
                return Some(home.join(".zshrc"));
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Some(home.join(".bashrc"));
            }
        }
        None
    }
}

#[cfg(unix)]
fn unix_line_for_rc(_rc_path: &Path, install_dir: &Path, home: &Path) -> String {
    unix_path_line(install_dir, home)
}

#[cfg(unix)]
pub fn repair_unix_shell_rc(
    rc_path: &Path,
    install_dir: &Path,
    home: &Path,
) -> anyhow::Result<RepairPathReport> {
    let line = unix_line_for_rc(rc_path, install_dir, home);
    let existing = fs::read_to_string(rc_path).unwrap_or_default();
    if existing.contains(&line) || existing.contains(&install_dir.to_string_lossy().to_string()) {
        return Ok(RepairPathReport {
            repaired: false,
            already_present: true,
            disabled: false,
            install_dir: Some(install_dir.to_path_buf()),
            shell_rc: Some(rc_path.to_path_buf()),
            backup_path: None,
            message: "PATH already contains axhub install directory".into(),
            error: None,
        });
    }

    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup_path = if rc_path.exists() {
        let backup = rc_path.with_extension(format!(
            "{}axhub.bak",
            rc_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| format!("{ext}."))
                .unwrap_or_default()
        ));
        fs::copy(rc_path, &backup)?;
        Some(backup)
    } else {
        None
    };

    let prefix = if existing.ends_with('\n') || existing.is_empty() {
        ""
    } else {
        "\n"
    };
    let updated = format!("{existing}{prefix}\n# axhub CLI PATH (added by axhub repair)\n{line}\n");
    fs::write(rc_path, updated)?;

    Ok(RepairPathReport {
        repaired: true,
        already_present: false,
        disabled: false,
        install_dir: Some(install_dir.to_path_buf()),
        shell_rc: Some(rc_path.to_path_buf()),
        backup_path,
        message: "PATH repair line was written".into(),
        error: None,
    })
}

#[cfg(windows)]
#[derive(Debug, Deserialize)]
struct WindowsRepairScriptOutput {
    repaired: bool,
    already_present: bool,
    backup_path: Option<String>,
    message: Option<String>,
    error: Option<String>,
}

#[cfg(windows)]
fn repair_windows_user_path(install_dir: &Path) -> anyhow::Result<RepairPathReport> {
    let dir = install_dir.to_string_lossy().replace("'@", "' + '@");
    let script = r#"
$ErrorActionPreference = 'Stop'
try {
  $InstallDir = [string]::Copy(@'
__INSTALL_DIR__
'@).Trim()
  $Key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
  $RawPath = $Key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
  if ($null -eq $RawPath) { $RawPath = '' }
  $Kind = try { $Key.GetValueKind('Path') } catch { [Microsoft.Win32.RegistryValueKind]::ExpandString }
  $TargetRaw = $InstallDir.TrimEnd('\').ToLowerInvariant()
  $TargetExpanded = [Environment]::ExpandEnvironmentVariables($InstallDir).TrimEnd('\').ToLowerInvariant()
  $Already = $false
  foreach ($Part in ($RawPath -split ';' | Where-Object { $_ -ne '' })) {
    $RawPart = $Part.TrimEnd('\').ToLowerInvariant()
    $ExpandedPart = [Environment]::ExpandEnvironmentVariables($Part).TrimEnd('\').ToLowerInvariant()
    if (($RawPart -eq $TargetRaw) -or ($ExpandedPart -eq $TargetExpanded)) {
      $Already = $true
      break
    }
  }
  if ($Already) {
    [pscustomobject]@{
      repaired = $false
      already_present = $true
      backup_path = $null
      message = 'User PATH already contains axhub install directory'
      error = $null
    } | ConvertTo-Json -Compress
    exit 0
  }
  $BackupRoot = if ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'axhub' } else { Join-Path $env:TEMP 'axhub' }
  New-Item -ItemType Directory -Force -Path $BackupRoot | Out-Null
  $Stamp = Get-Date -Format 'yyyyMMddTHHmmssfffffff'
  $BackupPath = Join-Path $BackupRoot "path-backup-$Stamp.txt"
  @(
    "kind=$Kind"
    "path=$RawPath"
  ) | Set-Content -Path $BackupPath -Encoding UTF8
  $Next = if ([string]::IsNullOrWhiteSpace($RawPath)) { $InstallDir } else { "$RawPath;$InstallDir" }
  $Key.SetValue('Path', $Next, $Kind)
  if (($env:PATH -split ';') -notcontains $InstallDir) { $env:PATH = "$env:PATH;$InstallDir" }
  [pscustomobject]@{
    repaired = $true
    already_present = $false
    backup_path = $BackupPath
    message = 'User PATH was updated'
    error = $null
  } | ConvertTo-Json -Compress
} catch {
  [pscustomobject]@{
    repaired = $false
    already_present = $false
    backup_path = $null
    message = 'Windows PATH repair failed'
    error = $_.Exception.Message
  } | ConvertTo-Json -Compress
  exit 0
}
"#
    .replace("__INSTALL_DIR__", &dir);
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: WindowsRepairScriptOutput = serde_json::from_str(stdout.trim()).map_err(|err| {
        anyhow::anyhow!(
            "Windows PATH repair returned non-JSON output: {err}; stdout={}",
            stdout.trim()
        )
    })?;
    let repaired = parsed.repaired;
    let already_present = parsed.already_present;
    if repaired || already_present {
        add_install_dir_to_current_path(install_dir);
    }
    let message = parsed.message.unwrap_or_else(|| {
        if repaired {
            "User PATH was updated".into()
        } else {
            "Windows PATH repair did not make changes".into()
        }
    });
    Ok(RepairPathReport {
        repaired,
        already_present,
        disabled: false,
        install_dir: Some(install_dir.to_path_buf()),
        shell_rc: None,
        backup_path: parsed.backup_path.map(PathBuf::from),
        message,
        error: parsed.error,
    })
}

pub fn repair_path(explicit_dir: Option<PathBuf>) -> RepairPathReport {
    if std::env::var("AXHUB_DISABLE_PATH_REPAIR").as_deref() == Ok("1") {
        return RepairPathReport::disabled();
    }

    let Some(install_dir) = find_installed_dir(explicit_dir) else {
        return RepairPathReport::no_install_dir(
            "No axhub install directory was found. Install axhub first, then run repair again."
                .into(),
        );
    };

    if let Some(path) = std::env::var_os("PATH") {
        let current_path = path.to_string_lossy();
        if path_contains_dir(&current_path, &install_dir) {
            return RepairPathReport {
                repaired: false,
                already_present: true,
                disabled: false,
                install_dir: Some(install_dir),
                shell_rc: None,
                backup_path: None,
                message: "Current PATH already contains axhub install directory".into(),
                error: None,
            };
        }
    }

    #[cfg(unix)]
    {
        let Some(home) = home_dir() else {
            return RepairPathReport::no_install_dir(
                "HOME is not set; cannot pick shell rc".into(),
            );
        };
        let Some(rc_path) = default_unix_shell_rc(&home) else {
            return RepairPathReport::no_install_dir(
                "Automatic PATH repair supports zsh and bash. Add the axhub install directory to PATH manually for this shell.".into(),
            );
        };
        return match repair_unix_shell_rc(&rc_path, &install_dir, &home) {
            Ok(report) => {
                if report.repaired || report.already_present {
                    add_install_dir_to_current_path(&install_dir);
                }
                report
            }
            Err(error) => RepairPathReport::error(Some(install_dir), Some(rc_path), error),
        };
    }

    #[cfg(windows)]
    {
        return match repair_windows_user_path(&install_dir) {
            Ok(report) => report,
            Err(error) => RepairPathReport::error(Some(install_dir), None, error.into()),
        };
    }

    #[allow(unreachable_code)]
    RepairPathReport::no_install_dir("Unsupported OS for PATH repair".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROCESS_ENV_LOCK;

    struct EnvGuard {
        key: &'static str,
        old: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let old = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, old }
        }

        fn remove(key: &'static str) -> Self {
            let old = std::env::var_os(key);
            std::env::remove_var(key);
            Self { key, old }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.old.take() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn find_installed_dir_uses_explicit_dir_without_probe() {
        let dir = PathBuf::from("/tmp/axhub-explicit-test-dir");

        assert_eq!(find_installed_dir(Some(dir.clone())), Some(dir));
    }

    #[test]
    fn disabled_report_is_safe_noop_json_shape() {
        let report = RepairPathReport::disabled();

        assert!(!report.repaired);
        assert!(!report.already_present);
        assert!(report.disabled);
        assert!(report.install_dir.is_none());
        assert!(report.shell_rc.is_none());
        assert!(report.backup_path.is_none());
        assert!(report.error.is_none());
        assert!(report.message.contains("AXHUB_DISABLE_PATH_REPAIR"));
    }

    #[test]
    fn error_report_preserves_target_context() {
        let install_dir = PathBuf::from("/tmp/axhub-install-error");
        let shell_rc = PathBuf::from("/tmp/axhub-shell-error/.zshrc");
        let report = RepairPathReport::error(
            Some(install_dir.clone()),
            Some(shell_rc.clone()),
            anyhow::anyhow!("permission denied"),
        );

        assert!(!report.repaired);
        assert!(!report.already_present);
        assert!(!report.disabled);
        assert_eq!(report.install_dir.as_deref(), Some(install_dir.as_path()));
        assert_eq!(report.shell_rc.as_deref(), Some(shell_rc.as_path()));
        assert_eq!(report.error.as_deref(), Some("permission denied"));
    }

    #[test]
    fn path_contains_dir_uses_path_boundaries() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("bin");
        let near_match = temp.path().join("bin-extra");
        let joined = std::env::join_paths([near_match.as_path(), install_dir.as_path()]).unwrap();
        let joined = joined.to_string_lossy();

        assert!(path_contains_dir(&joined, &install_dir));
        assert!(!path_contains_dir(
            &near_match.to_string_lossy(),
            &install_dir
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unix_path_line_escapes_quotes_for_custom_dir() {
        let home = Path::new("/tmp/home");
        let install_dir = Path::new("/tmp/axhub\"quoted/bin");

        assert_eq!(
            unix_path_line(install_dir, home),
            "export PATH=\"/tmp/axhub\\\"quoted/bin:$PATH\""
        );
    }

    #[cfg(unix)]
    #[test]
    fn default_unix_shell_rc_selects_supported_shells_and_defaults() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();

        let _shell = EnvGuard::set("SHELL", "/bin/zsh");
        assert_eq!(default_unix_shell_rc(home), Some(home.join(".zshrc")));
        drop(_shell);

        let _shell = EnvGuard::set("SHELL", "/usr/bin/bash");
        assert_eq!(default_unix_shell_rc(home), Some(home.join(".bashrc")));
        drop(_shell);

        let _shell = EnvGuard::remove("SHELL");
        #[cfg(target_os = "macos")]
        assert_eq!(default_unix_shell_rc(home), Some(home.join(".zshrc")));
        #[cfg(not(target_os = "macos"))]
        assert_eq!(default_unix_shell_rc(home), Some(home.join(".bashrc")));
    }

    #[cfg(unix)]
    #[test]
    fn repair_unix_shell_rc_creates_new_rc_without_backup() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let install_dir = home.join(".axhub/bin");
        let rc_path = home.join(".bashrc");

        let report = repair_unix_shell_rc(&rc_path, &install_dir, &home).unwrap();
        let content = fs::read_to_string(&rc_path).unwrap();

        assert!(report.repaired);
        assert!(!report.already_present);
        assert!(report.backup_path.is_none());
        assert!(content.contains("# axhub CLI PATH"));
        assert!(content.contains(r#"export PATH="$HOME/.axhub/bin:$PATH""#));
    }

    #[cfg(unix)]
    #[test]
    fn repair_unix_shell_rc_detects_absolute_existing_entry() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let install_dir = home.join("custom/bin");
        let rc_path = home.join(".zshrc");
        fs::create_dir_all(&home).unwrap();
        fs::write(
            &rc_path,
            format!("export PATH=\"{}:$PATH\"\n", install_dir.to_string_lossy()),
        )
        .unwrap();

        let report = repair_unix_shell_rc(&rc_path, &install_dir, &home).unwrap();

        assert!(!report.repaired);
        assert!(report.already_present);
        assert!(report.backup_path.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn repair_path_honors_disable_env() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _disable = EnvGuard::set("AXHUB_DISABLE_PATH_REPAIR", "1");

        let report = repair_path(Some(PathBuf::from("/tmp/axhub-disabled")));

        assert!(report.disabled);
        assert!(!report.repaired);
    }

    #[cfg(unix)]
    #[test]
    fn repair_path_reports_current_path_already_present() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("bin");
        let _disable = EnvGuard::remove("AXHUB_DISABLE_PATH_REPAIR");
        let _path = EnvGuard::set(
            "PATH",
            std::env::join_paths([install_dir.as_path()]).unwrap(),
        );

        let report = repair_path(Some(install_dir.clone()));

        assert!(!report.repaired);
        assert!(report.already_present);
        assert_eq!(report.install_dir.as_deref(), Some(install_dir.as_path()));
        assert!(report.shell_rc.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn repair_path_writes_default_rc_for_explicit_dir() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let install_dir = home.join(".axhub/bin");
        let _disable = EnvGuard::remove("AXHUB_DISABLE_PATH_REPAIR");
        let _home = EnvGuard::set("HOME", &home);
        let _shell = EnvGuard::set("SHELL", "/bin/zsh");
        let _path = EnvGuard::set("PATH", "");

        let report = repair_path(Some(install_dir.clone()));
        let rc_path = home.join(".zshrc");
        let content = fs::read_to_string(&rc_path).unwrap();

        assert!(report.repaired);
        assert_eq!(report.install_dir.as_deref(), Some(install_dir.as_path()));
        assert_eq!(report.shell_rc.as_deref(), Some(rc_path.as_path()));
        assert!(content.contains(r#"export PATH="$HOME/.axhub/bin:$PATH""#));
        assert!(path_contains_dir(
            &std::env::var("PATH").unwrap_or_default(),
            &install_dir
        ));
    }

    #[cfg(unix)]
    #[test]
    fn repair_path_reports_missing_home() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("bin");
        let _disable = EnvGuard::remove("AXHUB_DISABLE_PATH_REPAIR");
        let _home = EnvGuard::remove("HOME");
        let _userprofile = EnvGuard::remove("USERPROFILE");
        let _path = EnvGuard::set("PATH", "");

        let report = repair_path(Some(install_dir));

        assert!(!report.repaired);
        assert!(!report.already_present);
        assert!(report.message.contains("HOME is not set"));
    }

    #[cfg(unix)]
    #[test]
    fn repair_path_reports_unsupported_shell_without_mutation() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let install_dir = home.join(".axhub/bin");
        let _disable = EnvGuard::remove("AXHUB_DISABLE_PATH_REPAIR");
        let _home = EnvGuard::set("HOME", &home);
        let _shell = EnvGuard::set("SHELL", "/usr/bin/fish");
        let _path = EnvGuard::set("PATH", "");

        let report = repair_path(Some(install_dir));

        assert!(!report.repaired);
        assert!(!report.already_present);
        assert!(report.shell_rc.is_none());
        assert!(report.message.contains("supports zsh and bash"));
        assert!(!home.join(".zshrc").exists());
        assert!(!home.join(".bashrc").exists());
    }

    #[test]
    fn repair_path_reports_missing_install_dir_when_no_candidate_exists() {
        let _lock = PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let _disable = EnvGuard::remove("AXHUB_DISABLE_PATH_REPAIR");
        let _install = EnvGuard::remove("AXHUB_INSTALL_DIR");
        let _dist = EnvGuard::remove("CARGO_DIST_INSTALL_DIR");
        let _cargo = EnvGuard::remove("CARGO_HOME");
        let _home = EnvGuard::set("HOME", temp.path().join("home"));
        let _userprofile = EnvGuard::remove("USERPROFILE");
        let _path = EnvGuard::set("PATH", "");

        let report = repair_path(None);

        assert!(!report.repaired);
        assert!(!report.already_present);
        assert!(report.install_dir.is_none());
        assert!(report.message.contains("No axhub install directory"));
    }
}

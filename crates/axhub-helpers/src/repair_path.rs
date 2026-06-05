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
fn unix_line_for_rc(rc_path: &Path, install_dir: &Path, home: &Path) -> String {
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

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const FILE_MODE_PRIVATE: u32 = 0o600;
pub const DIR_MODE_PRIVATE: u32 = 0o700;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub token_file: PathBuf,
    pub last_deploy_file: PathBuf,
    pub state_dir: PathBuf,
}

impl RuntimePaths {
    pub fn current() -> Option<Self> {
        Some(Self {
            token_file: token_file()?,
            last_deploy_file: last_deploy_file()?,
            state_dir: state_dir()?,
        })
    }
}

pub fn token_file() -> Option<PathBuf> {
    config_base_dir().map(|base| base.join("axhub-plugin").join("token"))
}

pub fn last_deploy_file() -> Option<PathBuf> {
    cache_base_dir().map(|base| base.join("axhub-plugin").join("last-deploy.json"))
}

pub fn state_dir() -> Option<PathBuf> {
    state_base_dir().map(|base| base.join("axhub-plugin"))
}

/// Shared non-plugin state root used by helper subsystems that need a stable,
/// user-private location outside the plugin cache/config tree.
///
/// This intentionally preserves the historical `$XDG_STATE_HOME/axhub` (or
/// `$HOME/.local/state/axhub`) location that older approval-key helpers used so
/// audit/diagnose data stays in place while the legacy approval gate is removed.
pub fn state_root() -> PathBuf {
    state_root_from(env_path("XDG_STATE_HOME"), home_dir(), stable_process_dir())
}

/// Path to the per-version SessionStart welcome marker. Presence means the
/// magical-moment message for that version has already been shown.
pub fn welcome_marker_path(version: &str) -> Option<PathBuf> {
    state_dir().map(|dir| dir.join(format!(".v{version}-welcome-shown")))
}

/// Phase 26 PR 26.1b — directory housing one NDJSON file per deploy.
/// `$XDG_STATE_HOME/axhub-plugin/deploy-events/`. The directory is created on
/// first append by `event_log::append_event`.
pub fn deploy_events_dir() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("deploy-events"))
}

/// Phase 25 PR 25.6 — cooldown marker for `axhub-helpers doctor` size
/// warnings. File mtime is the canonical "last warning emitted" timestamp;
/// the doctor reads + compares against now() to decide whether to re-warn.
pub fn doctor_cooldown_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("doctor-cooldown.json"))
}

/// TTL cache for the last-fetched latest plugin release.
/// `$XDG_CACHE_HOME/axhub-plugin/plugin-latest.json`. Written atomically by the
/// `plugin-latest-fetch-bg` background fetch, read by the prompt-route nudge.
pub fn plugin_latest_cache_path() -> Option<PathBuf> {
    cache_base_dir().map(|base| base.join("axhub-plugin").join("plugin-latest.json"))
}

/// Per-version drift-nudge marker. Presence means the update nudge for `version`
/// already fired (once-per-version dedup).
pub fn plugin_drift_nudge_marker_path(version: &str) -> Option<PathBuf> {
    state_dir().map(|dir| dir.join(format!(".plugin-drift-nudged-v{version}")))
}

/// Permanent opt-out marker (DX "그만 볼래요"). Presence suppresses the drift
/// nudge for every version.
pub fn plugin_drift_optout_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("plugin-drift-optout"))
}

fn config_base_dir() -> Option<PathBuf> {
    config_base_dir_from(env_path("XDG_CONFIG_HOME"), home_dir())
}

fn cache_base_dir() -> Option<PathBuf> {
    cache_base_dir_from(env_path("XDG_CACHE_HOME"), home_dir())
}

fn state_base_dir() -> Option<PathBuf> {
    state_base_dir_from(env_path("XDG_STATE_HOME"), home_dir())
}

fn config_base_dir_from(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    xdg_config_home.or_else(|| home.map(|home| home.join(".config")))
}

fn cache_base_dir_from(xdg_cache_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_cache_home.or_else(|| home.map(|home| home.join(".cache")))
}

fn state_base_dir_from(xdg_state_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_state_home.or_else(|| home.map(|home| home.join(".local").join("state")))
}

fn state_root_from(
    xdg_state_home: Option<PathBuf>,
    home: Option<PathBuf>,
    stable_fallback: PathBuf,
) -> PathBuf {
    xdg_state_home
        .or_else(|| home.map(|home| home.join(".local").join("state")))
        .unwrap_or_else(|| stable_fallback.join(".local").join("state"))
        .join("axhub")
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    home_dir_from(
        env_path("HOME"),
        env_path("USERPROFILE"),
        env_path("HOMEDRIVE"),
        env_path("HOMEPATH"),
    )
}

fn home_dir_from(
    home: Option<PathBuf>,
    userprofile: Option<PathBuf>,
    homedrive: Option<PathBuf>,
    homepath: Option<PathBuf>,
) -> Option<PathBuf> {
    home.or(userprofile).or_else(|| {
        let mut home = homedrive?;
        home.push(homepath?);
        Some(home)
    })
}

fn stable_process_dir() -> PathBuf {
    std::env::current_dir()
        .or_else(|_| {
            std::env::current_exe().map(|exe| {
                exe.parent()
                    .map(PathBuf::from)
                    .unwrap_or_else(stable_root_dir)
            })
        })
        .unwrap_or_else(|_| stable_root_dir())
}

#[cfg(windows)]
fn stable_root_dir() -> PathBuf {
    std::env::var_os("SystemDrive")
        .map(PathBuf::from)
        .map(|mut drive| {
            drive.push(std::path::MAIN_SEPARATOR.to_string());
            drive
        })
        .unwrap_or_else(|| PathBuf::from(r"C:\"))
}

#[cfg(not(windows))]
fn stable_root_dir() -> PathBuf {
    PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
}

pub fn read_private_file(path: &Path) -> anyhow::Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        !meta.file_type().is_symlink(),
        "private file path is a symlink"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode() & 0o777;
        anyhow::ensure!(mode & 0o077 == 0, "private file is group/world-readable");
    }
    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn write_private_file_no_follow(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        anyhow::ensure!(
            !meta.file_type().is_symlink(),
            "private file path is a symlink"
        );
    }
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(FILE_MODE_PRIVATE);
        opts.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = opts.open(path)?;
    file.write_all(bytes)?;
    file.sync_all().ok();
    set_private_file_mode(path).ok();
    Ok(())
}

pub fn set_private_file_mode(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(FILE_MODE_PRIVATE))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

pub fn set_private_dir_mode(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(DIR_MODE_PRIVATE))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_paths_override_home_contracts() {
        assert_eq!(
            config_base_dir_from(
                Some(PathBuf::from("/xdg-config")),
                Some(PathBuf::from("/home/user"))
            ),
            Some(PathBuf::from("/xdg-config"))
        );
        assert_eq!(
            cache_base_dir_from(
                Some(PathBuf::from("/xdg-cache")),
                Some(PathBuf::from("/home/user"))
            ),
            Some(PathBuf::from("/xdg-cache"))
        );
        assert_eq!(
            state_base_dir_from(
                Some(PathBuf::from("/xdg-state")),
                Some(PathBuf::from("/home/user"))
            ),
            Some(PathBuf::from("/xdg-state"))
        );
    }

    #[test]
    fn home_fallback_matches_plugin_paths_on_unix_and_windows() {
        assert_eq!(
            config_base_dir_from(None, Some(PathBuf::from("/home/user"))),
            Some(PathBuf::from("/home/user/.config"))
        );
        assert_eq!(
            cache_base_dir_from(None, Some(PathBuf::from("/home/user"))),
            Some(PathBuf::from("/home/user/.cache"))
        );
        assert_eq!(
            state_base_dir_from(None, Some(PathBuf::from("/home/user"))),
            Some(PathBuf::from("/home/user/.local/state"))
        );
    }

    #[test]
    fn runtime_paths_current_resolves_all_plugin_paths() {
        let paths = RuntimePaths::current().expect("test runner has a home directory");

        assert!(paths.token_file.ends_with("axhub-plugin/token"));
        assert!(paths
            .last_deploy_file
            .ends_with("axhub-plugin/last-deploy.json"));
        assert!(paths.state_dir.ends_with("axhub-plugin"));
    }

    #[test]
    fn state_root_ignores_empty_xdg_and_uses_home() {
        assert_eq!(
            state_root_from(
                None,
                Some(PathBuf::from("/home/alice")),
                PathBuf::from("/cwd")
            ),
            PathBuf::from("/home/alice/.local/state/axhub")
        );
    }

    #[test]
    fn home_dir_supports_userprofile_when_home_missing() {
        assert_eq!(
            home_dir_from(
                None,
                Some(PathBuf::from("/Users/alice")),
                Some(PathBuf::from("ignored-drive")),
                Some(PathBuf::from("ignored-path")),
            ),
            Some(PathBuf::from("/Users/alice"))
        );
    }

    #[test]
    fn home_dir_supports_homedrive_homepath_when_home_and_userprofile_missing() {
        assert_eq!(
            home_dir_from(
                None,
                None,
                Some(PathBuf::from("C:")),
                Some(PathBuf::from("Users\\alice")),
            ),
            Some(PathBuf::from("C:").join("Users\\alice"))
        );
    }
}

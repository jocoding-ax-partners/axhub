use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

pub const HMAC_KEY_BYTES: usize = 32;
pub const FILE_MODE_PRIVATE: u32 = 0o600;
pub const DIR_MODE_PRIVATE: u32 = 0o700;

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
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

pub fn state_root() -> PathBuf {
    state_root_from(env_path("XDG_STATE_HOME"), home_dir(), stable_process_dir())
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
pub fn runtime_root() -> PathBuf {
    // When XDG_RUNTIME_DIR is set (typically Linux) both the Bash tool and the
    // hook subprocess inherit the same value, so consent mint/read agree. When
    // it is unset (macOS Claude Code), the old `std::env::temp_dir()` fallback
    // read each process's own `$TMPDIR`, which the harness sets differently per
    // process — so the mint dir and the hook read dir diverged and consent was
    // never found (PreToolUse deny). Fall back instead to a HOME-anchored,
    // process-stable directory: the same `state_root()` the HMAC key already
    // uses and that both processes provably resolve identically.
    match std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg).join("axhub"),
        None => state_root().join("runtime"),
    }
}
pub fn hmac_key_path() -> PathBuf {
    state_root().join("hmac-key")
}
pub fn session_id() -> anyhow::Result<String> {
    std::env::var("CLAUDE_SESSION_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("CLAUDE_SESSION_ID is required for consent token mint/verify")
        })
}
pub fn token_file_path(sid: &str) -> PathBuf {
    runtime_root().join(format!("consent-{sid}.json"))
}
pub fn pending_token_file_path(token_id: &str) -> PathBuf {
    runtime_root().join(format!("consent-pending-{token_id}.json"))
}

pub fn load_or_mint_key() -> anyhow::Result<[u8; HMAC_KEY_BYTES]> {
    let path = hmac_key_path();
    match read_private_file(&path) {
        Ok(buf) => {
            anyhow::ensure!(buf.len() == HMAC_KEY_BYTES, "hmac-key has wrong length");
            let mut out = [0u8; HMAC_KEY_BYTES];
            out.copy_from_slice(&buf);
            Ok(out)
        }
        Err(e)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound) =>
        {
            fs::create_dir_all(state_root())?;
            set_private_dir_mode(&state_root()).ok();
            let mut key = [0u8; HMAC_KEY_BYTES];
            getrandom::getrandom(&mut key)
                .map_err(|e| anyhow::anyhow!("getrandom failed: {:?}", e))?;
            write_private_file_no_follow(&path, &key)?;
            Ok(key)
        }
        Err(e) => Err(e),
    }
}

pub fn read_private_file(path: &PathBuf) -> anyhow::Result<Vec<u8>> {
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

pub fn write_private_file_no_follow(path: &PathBuf, bytes: &[u8]) -> anyhow::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        anyhow::ensure!(
            !meta.file_type().is_symlink(),
            "consent token path is a symlink"
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

pub fn set_private_file_mode(path: &PathBuf) -> std::io::Result<()> {
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

pub fn set_private_dir_mode(path: &PathBuf) -> std::io::Result<()> {
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

    #[test]
    fn state_root_fallback_is_stable_and_absolute() {
        let fallback = std::env::current_dir().unwrap();
        let root = state_root_from(None, None, fallback.clone());
        assert_eq!(root, fallback.join(".local").join("state").join("axhub"));
        assert!(root.is_absolute());
    }
}

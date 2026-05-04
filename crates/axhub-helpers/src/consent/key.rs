use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

pub const HMAC_KEY_BYTES: usize = 32;
pub const FILE_MODE_PRIVATE: u32 = 0o600;
pub const DIR_MODE_PRIVATE: u32 = 0o700;

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
pub fn state_root() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/state"))
        .join("axhub")
}
pub fn runtime_root() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("axhub")
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

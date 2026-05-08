//! Phase 1 — `session-bundle.json` reader/writer.
//!
//! Spec: `.plan/deploy-time-reduction/phase-1-rest-dedup-statusline.md` §3.2.
//!
//! SessionStart hook serializes preflight output to a per-session bundle so the
//! deploy SKILL can reuse it without re-invoking preflight. Writes are
//! tempfile + rename to keep readers from observing partial JSON. Reads enforce
//! a 5-minute TTL and validate `schema_version` so stale or mismatched files
//! fall back to an empty bundle rather than serving wrong data.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

pub const SESSION_BUNDLE_SCHEMA_VERSION: &str = "session-bundle/v1";
pub const SESSION_BUNDLE_TTL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthStatusBundle {
    pub ok: bool,
    pub user_email: Option<String>,
    pub user_id: Option<i64>,
    pub expires_at: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LastDeployBundle {
    pub deployment_id: String,
    pub status: String,
    pub commit_sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionBundle {
    pub schema_version: String,
    pub auth_status: AuthStatusBundle,
    pub current_app: Option<String>,
    pub current_env: Option<String>,
    pub last_deploy: Option<LastDeployBundle>,
    pub plugin_version: String,
    pub helper_version: String,
    pub written_at: String,
}

impl SessionBundle {
    pub fn empty() -> Self {
        SessionBundle {
            schema_version: SESSION_BUNDLE_SCHEMA_VERSION.to_string(),
            auth_status: AuthStatusBundle {
                ok: false,
                user_email: None,
                user_id: None,
                expires_at: None,
                scopes: Vec::new(),
            },
            current_app: None,
            current_env: None,
            last_deploy: None,
            plugin_version: env!("CARGO_PKG_VERSION").to_string(),
            helper_version: env!("CARGO_PKG_VERSION").to_string(),
            written_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }
}

pub fn write_session_bundle(bundle: &SessionBundle, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let tmp_path = path.with_extension("json.tmp");
    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp_path)?;
    let json = serde_json::to_string_pretty(bundle)?;
    f.write_all(json.as_bytes())?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    drop(f);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn read_session_bundle(path: &Path) -> anyhow::Result<SessionBundle> {
    read_session_bundle_at(path, SystemTime::now())
}

pub fn read_session_bundle_at(path: &Path, now: SystemTime) -> anyhow::Result<SessionBundle> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let elapsed = now.duration_since(modified).unwrap_or_default();
    if elapsed.as_secs() > SESSION_BUNDLE_TTL_SECS {
        return Err(anyhow::anyhow!(
            "session-bundle expired after {SESSION_BUNDLE_TTL_SECS}s"
        ));
    }
    let raw = fs::read_to_string(path)?;
    let bundle: SessionBundle = serde_json::from_str(&raw)?;
    if bundle.schema_version != SESSION_BUNDLE_SCHEMA_VERSION {
        return Err(anyhow::anyhow!(
            "session-bundle schema mismatch: {}",
            bundle.schema_version
        ));
    }
    Ok(bundle)
}

pub fn read_session_bundle_or_empty(path: &Path) -> SessionBundle {
    read_session_bundle(path).unwrap_or_else(|_| SessionBundle::empty())
}

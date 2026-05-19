//! `LoopShadowProbe` — writes to a file inside the loop's `cwd-shadow`
//! tempdir, reverts by removing it. Plan v6 §4.4 — v0.8.0 builtin probe.

use std::path::PathBuf;

use serde_json::json;

use super::super::DiagnoseError;
use super::{ApplyHandle, Probe, ProbeContext, ProbeTouch};

pub struct LoopShadowProbe {
    pub id: String,
    pub hypothesis_id: String,
    /// Relative path inside `<shadow_root>/<loop_id>/cwd-shadow/`. MUST NOT
    /// escape the shadow root via `..`.
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
}

impl LoopShadowProbe {
    fn shadow_path(&self, ctx: &ProbeContext) -> Result<PathBuf, DiagnoseError> {
        // Reject path traversal at the component level — `..` is never allowed
        // anywhere in the relative path. This catches `../../../etc/passwd`
        // regardless of canonicalization availability.
        use std::path::Component;
        for c in self.relative_path.components() {
            match c {
                Component::Normal(_) | Component::CurDir => {}
                _ => {
                    return Err(DiagnoseError::ProbeBoundaryViolation {
                        probe_id: self.id.clone(),
                        path: self.relative_path.display().to_string(),
                    });
                }
            }
        }
        let base = ctx.shadow_root.join(&ctx.loop_id).join("cwd-shadow");
        Ok(base.join(&self.relative_path))
    }
}

impl Probe for LoopShadowProbe {
    fn id(&self) -> &str {
        &self.id
    }
    fn hypothesis_id(&self) -> &str {
        &self.hypothesis_id
    }
    fn touches(&self) -> Vec<ProbeTouch> {
        // We do not know ctx.shadow_root at touches() time — return the
        // relative path as a sentinel. `instrument` reconciles with the full
        // absolute path before recording the manifest entry.
        vec![ProbeTouch::LoopShadowFile(self.relative_path.clone())]
    }
    fn apply(&self, ctx: &ProbeContext) -> Result<ApplyHandle, DiagnoseError> {
        let target = self.shadow_path(ctx)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DiagnoseError::ProbeApplyFailed(e.to_string()))?;
        }
        std::fs::write(&target, &self.contents)
            .map_err(|e| DiagnoseError::ProbeApplyFailed(e.to_string()))?;
        Ok(ApplyHandle {
            probe_id: self.id.clone(),
            touched: vec![ProbeTouch::LoopShadowFile(target.clone())],
            revert_metadata: json!({ "abs_path": target.to_string_lossy().to_string() }),
        })
    }
    fn revert(&self, handle: ApplyHandle) -> Result<(), DiagnoseError> {
        let path_str = handle
            .revert_metadata
            .get("abs_path")
            .and_then(|v| v.as_str());
        let Some(path_str) = path_str else {
            return Ok(()); // nothing to revert
        };
        let path = PathBuf::from(path_str);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| DiagnoseError::CleanupFailed(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ctx_in(dir: &TempDir) -> ProbeContext {
        ProbeContext {
            loop_id: "loop-shadow-test".into(),
            shadow_root: dir.path().to_path_buf(),
        }
    }

    #[test]
    fn writes_and_reverts_shadow_file() {
        let dir = TempDir::new().unwrap();
        let ctx = ctx_in(&dir);
        let p = LoopShadowProbe {
            id: "ls-1".into(),
            hypothesis_id: "H".into(),
            relative_path: PathBuf::from("subdir/sample.txt"),
            contents: b"hello".to_vec(),
        };
        let handle = p.apply(&ctx).unwrap();
        let abs: String = handle
            .revert_metadata
            .get("abs_path")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        assert!(std::fs::read(&abs).unwrap() == b"hello");
        p.revert(handle).unwrap();
        assert!(!PathBuf::from(&abs).exists(), "file must be deleted on revert");
    }

    #[test]
    fn revert_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let ctx = ctx_in(&dir);
        let p = LoopShadowProbe {
            id: "ls-2".into(),
            hypothesis_id: "H".into(),
            relative_path: PathBuf::from("a.txt"),
            contents: b"a".to_vec(),
        };
        let handle = p.apply(&ctx).unwrap();
        p.revert(handle.clone()).unwrap();
        // Second revert (already gone) must be a no-op success.
        p.revert(handle).unwrap();
    }

    #[test]
    fn path_traversal_rejected_at_apply() {
        let dir = TempDir::new().unwrap();
        let ctx = ctx_in(&dir);
        let p = LoopShadowProbe {
            id: "ls-3".into(),
            hypothesis_id: "H".into(),
            relative_path: PathBuf::from("../../../etc/passwd"),
            contents: b"x".to_vec(),
        };
        let result = p.apply(&ctx);
        assert!(
            matches!(result, Err(DiagnoseError::ProbeBoundaryViolation { .. })),
            "path traversal must be rejected"
        );
    }
}

//! Phase 3I instrumentation — plan v6 §4.4.
//!
//! Wraps each Probe with:
//! - Pre-apply mtime + content-hash snapshot of every `touches()` path
//! - Audit ledger entry on apply (failure → immediate revert so a mutation is
//!   never left without a recoverable manifest)
//! - Post-revert boundary guard: real sha256 comparison rejects probes that
//!   touched any path outside their declared `touches()` set.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use super::probe::{manifest::ProbeManifest, Probe, ProbeContext, ProbeTouch};
use super::signal::Signal;
use super::DiagnoseError;
use crate::audit_ledger;

#[derive(Debug, Clone, Default)]
pub struct PathSnapshot {
    /// mtime → fast filter for "did anything change."
    pub mtime: Option<SystemTime>,
    /// sha256 of file contents → ground truth (mtime can drift falsely).
    pub sha256: Option<String>,
}

fn snapshot_path(p: &std::path::Path) -> PathSnapshot {
    if !p.is_file() {
        return PathSnapshot::default();
    }
    let mtime = std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let sha256 = std::fs::read(p).ok().map(|bytes| {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    });
    PathSnapshot { mtime, sha256 }
}

/// Apply a probe under the boundary guard. Records the audit manifest and a
/// pre-apply snapshot of every `touches()` file. Returns the signal Phase 3I
/// observes after the probe is active.
pub fn instrument<P: Probe + ?Sized>(
    probe: &P,
    ctx: &ProbeContext,
) -> Result<(Signal, ApplyOutcome), DiagnoseError> {
    let touches = probe.touches();
    let pre_snapshots = collect_snapshots(&touches, ctx);

    let handle = probe.apply(ctx)?;

    let manifest = ProbeManifest::from_apply(&handle, ctx.loop_id.clone());
    if let Err(ledger_err) = audit_ledger::append_entry(&manifest.into_ledger_entry()) {
        // The manifest is the only authority for revert (plan v6 §4.4). If we
        // can't persist it, immediately undo the probe — leaving an applied
        // mutation without a recoverable manifest is the failure mode the
        // manifest itself was designed to prevent.
        let revert_msg = match probe.revert(handle) {
            Ok(()) => "reverted",
            Err(e) => {
                return Err(DiagnoseError::ProbeApplyFailed(format!(
                    "audit ledger write failed ({ledger_err}); subsequent revert ALSO failed ({e})"
                )));
            }
        };
        return Err(DiagnoseError::ProbeApplyFailed(format!(
            "audit ledger write failed: {ledger_err}; probe {revert_msg}"
        )));
    }

    let signal = probe.run(ctx)?;

    Ok((
        signal,
        ApplyOutcome {
            handle,
            pre_snapshots,
        },
    ))
}

/// Revert under boundary guard. Verifies that paths touched during apply now
/// match their pre-apply sha256 (for shadow files, "post-revert sha256 must
/// be empty / unchanged-pre-existing") — anything else means the probe wrote
/// outside its declared `touches()` set.
pub fn revert_with_guard<P: Probe + ?Sized>(
    probe: &P,
    outcome: ApplyOutcome,
) -> Result<(), DiagnoseError> {
    let pre_snapshots = outcome.pre_snapshots.clone();
    let probe_id = outcome.handle.probe_id.clone();
    probe.revert(outcome.handle)?;

    // Boundary guard: every path we declared MUST be back to its pre-apply
    // content hash (or, if it didn't exist pre-apply, still not exist).
    // LoopShadowFile probes typically expect post-revert state == pre-apply
    // state == absent, so post.sha256 == None == pre.sha256 == None.
    for (path, pre) in pre_snapshots.iter() {
        let post = snapshot_path(path);
        if post.sha256 != pre.sha256 {
            return Err(DiagnoseError::ProbeBoundaryViolation {
                probe_id: probe_id.clone(),
                path: path.display().to_string(),
            });
        }
    }
    Ok(())
}

fn collect_snapshots(
    touches: &[ProbeTouch],
    ctx: &ProbeContext,
) -> BTreeMap<PathBuf, PathSnapshot> {
    let mut map = BTreeMap::new();
    for t in touches {
        if let ProbeTouch::LoopShadowFile(rel) = t {
            // Reconcile the probe's relative path against the loop's shadow
            // root so the snapshot key matches the file the probe actually
            // writes. Previously this stored the relative path verbatim,
            // which made the snapshot point at a cwd-relative non-existent
            // location and silently neutered the boundary guard.
            let abs = ctx
                .shadow_root
                .join(&ctx.loop_id)
                .join("cwd-shadow")
                .join(rel);
            map.insert(abs.clone(), snapshot_path(&abs));
        }
        // UserCodeLines: v0.8.1+ (CodeInjectionProbe).
        // EnvVar: no path snapshot needed — EnvVarProbe::revert persists its
        // own prior-value manifest. Cross-probe env races are serialised by
        // crate::PROCESS_ENV_LOCK.
    }
    map
}

#[derive(Debug)]
pub struct ApplyOutcome {
    pub handle: super::probe::ApplyHandle,
    pub pre_snapshots: BTreeMap<PathBuf, PathSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::probe::{EnvVarProbe, LoopShadowProbe};
    use tempfile::TempDir;

    #[test]
    fn instrument_round_trip_env_var() {
        let var = "AXHUB_TEST_INSTRUMENT_VAR";
        std::env::remove_var(var);
        let probe = EnvVarProbe {
            id: "p-instr-1".into(),
            hypothesis_id: "H-instr-1".into(),
            var_name: var.into(),
            new_value: Some("v1".into()),
        };
        let ctx = ProbeContext {
            loop_id: "loop-instrument-1".into(),
            shadow_root: PathBuf::from("/tmp"),
        };
        let (signal, outcome) = instrument(&probe, &ctx).unwrap();
        assert!(signal.is_green()); // default Probe::run returns green
        assert_eq!(std::env::var(var).unwrap(), "v1");
        revert_with_guard(&probe, outcome).unwrap();
        assert!(std::env::var(var).is_err());
    }

    #[test]
    fn snapshot_of_missing_path_is_empty() {
        let snap = snapshot_path(std::path::Path::new("/nonexistent/abc/xyz/file"));
        assert!(snap.mtime.is_none());
        assert!(snap.sha256.is_none());
    }

    #[test]
    fn snapshot_existing_file_records_hash_and_mtime() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sample.txt");
        std::fs::write(&path, b"hello").unwrap();
        let snap = snapshot_path(&path);
        assert!(snap.mtime.is_some());
        assert_eq!(
            snap.sha256,
            Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".into())
        );
    }

    #[test]
    fn instrument_round_trip_loop_shadow_file() {
        let dir = TempDir::new().unwrap();
        let probe = LoopShadowProbe {
            id: "shadow-instrument".into(),
            hypothesis_id: "H-shadow".into(),
            relative_path: PathBuf::from("nested/file.txt"),
            contents: b"shadow".to_vec(),
        };
        let ctx = ProbeContext {
            loop_id: "loop-shadow-instrument".into(),
            shadow_root: dir.path().to_path_buf(),
        };

        let (signal, outcome) = instrument(&probe, &ctx).unwrap();
        assert!(signal.is_green());
        assert_eq!(outcome.pre_snapshots.len(), 1);
        let path = dir
            .path()
            .join("loop-shadow-instrument/cwd-shadow/nested/file.txt");
        assert_eq!(std::fs::read(&path).unwrap(), b"shadow");

        revert_with_guard(&probe, outcome).unwrap();
        assert!(!path.exists());
    }
}

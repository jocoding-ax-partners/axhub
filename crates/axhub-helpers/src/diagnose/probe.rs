//! Phase 3I Probe trait — plan v6 §4.4.
//!
//! A Probe is a single-variable change applied to the environment to test a
//! hypothesis. Trait contract:
//! - `apply` must declare all touched paths via `touches()` BEFORE returning.
//! - `revert` must restore the world to its pre-apply state idempotently.
//! - `revert` MUST NOT mutate anything outside `touches()` — enforced at
//!   runtime by [`super::instrument`] via mtime + content hash check.
//!
//! v0.8.0 ships 2 builtin Probes:
//! - `EnvVarProbe` (env var snapshot + restore)
//! - `LoopShadowProbe` (tempdir manipulation)
//!
//! `CodeInjectionProbe` is intentionally v0.8.1+ (eng-review cross-consensus).

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::signal::Signal;
use super::DiagnoseError;

pub mod env_var;
pub mod loop_shadow;
pub mod manifest;

pub use env_var::EnvVarProbe;
pub use loop_shadow::LoopShadowProbe;

/// Categorization of what a probe writes to. Used by [`super::instrument`] to
/// enforce the runtime boundary guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeTouch {
    /// User-owned source file with a specific line range. Probe owns those
    /// lines exclusively. Plan v6 §4.4 — never used in v0.8.0
    /// (CodeInjectionProbe is v0.8.1+).
    UserCodeLines {
        path: PathBuf,
        start: u32,
        end: u32,
    },
    /// Env var. No file touch.
    EnvVar(String),
    /// File inside `~/.axhub/loops/<loop_id>/cwd-shadow/`. Always safe to
    /// delete on revert.
    LoopShadowFile(PathBuf),
}

/// Opaque handle returned by `apply` and consumed by `revert`. Carries the
/// information needed to undo the change idempotently.
#[derive(Debug, Clone)]
pub struct ApplyHandle {
    pub probe_id: String,
    pub touched: Vec<ProbeTouch>,
    pub revert_metadata: serde_json::Value,
}

/// Context passed to every probe invocation. Bounds the probe to a single
/// loop and gives it access to the loop's shadow directory.
#[derive(Debug, Clone)]
pub struct ProbeContext {
    pub loop_id: String,
    pub shadow_root: PathBuf,
}

/// Plan v6 §4.4 — single-variable diagnostic probe. Apply → observe → revert.
pub trait Probe: Send + Sync {
    /// Stable identifier (used in audit ledger entries).
    fn id(&self) -> &str;

    /// Which hypothesis this probe distinguishes.
    fn hypothesis_id(&self) -> &str;

    /// What this probe writes to. Must be declared BEFORE `apply` mutates
    /// the world — `super::instrument` uses it to record the pre-apply state.
    fn touches(&self) -> Vec<ProbeTouch>;

    /// Apply the probe. Returns an `ApplyHandle` that `revert` consumes.
    fn apply(&self, ctx: &ProbeContext) -> Result<ApplyHandle, DiagnoseError>;

    /// Re-run the loop signal after this probe is active. Default impl simply
    /// reports a green signal — concrete probes override to wire in a real
    /// loop_builder call.
    fn run(&self, _ctx: &ProbeContext) -> Result<Signal, DiagnoseError> {
        Ok(Signal::green(Duration::from_millis(0), self.id()))
    }

    /// Revert the change. MUST be idempotent (calling twice is harmless).
    /// MUST NOT touch any path outside `touches()`.
    fn revert(&self, handle: ApplyHandle) -> Result<(), DiagnoseError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoProbe {
        pub id: String,
    }

    impl Probe for EchoProbe {
        fn id(&self) -> &str {
            &self.id
        }
        fn hypothesis_id(&self) -> &str {
            "H-echo"
        }
        fn touches(&self) -> Vec<ProbeTouch> {
            vec![ProbeTouch::EnvVar("ECHO_PROBE".into())]
        }
        fn apply(&self, _ctx: &ProbeContext) -> Result<ApplyHandle, DiagnoseError> {
            Ok(ApplyHandle {
                probe_id: self.id.clone(),
                touched: self.touches(),
                revert_metadata: serde_json::json!({}),
            })
        }
        fn revert(&self, _handle: ApplyHandle) -> Result<(), DiagnoseError> {
            Ok(())
        }
    }

    #[test]
    fn trait_apply_revert_roundtrip() {
        let p = EchoProbe { id: "echo-1".into() };
        let ctx = ProbeContext {
            loop_id: "loop-test".into(),
            shadow_root: PathBuf::from("/tmp/shadow"),
        };
        let handle = p.apply(&ctx).unwrap();
        assert_eq!(handle.probe_id, "echo-1");
        assert_eq!(handle.touched.len(), 1);
        // Idempotent revert.
        p.revert(handle.clone()).unwrap();
        p.revert(handle).unwrap();
    }

    #[test]
    fn probe_touch_serde() {
        let t = ProbeTouch::EnvVar("PATH".into());
        let s = serde_json::to_string(&t).unwrap();
        let back: ProbeTouch = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn default_run_returns_green() {
        let p = EchoProbe { id: "echo-default".into() };
        let ctx = ProbeContext {
            loop_id: "loop-test".into(),
            shadow_root: PathBuf::from("/tmp/shadow"),
        };
        let sig = p.run(&ctx).unwrap();
        assert!(sig.is_green());
    }
}

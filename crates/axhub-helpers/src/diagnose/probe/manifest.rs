//! Probe manifest — the serialization layer between `Probe::apply` and the
//! `audit_ledger`. Plan v6 §4.4 + §13.B.
//!
//! Every probe apply writes a manifest entry into the audit ledger so the
//! Phase 5P cleanup can revert by reading manifests back, not by grep.

use serde::{Deserialize, Serialize};

use crate::audit_ledger::LedgerEntry;

use super::{ApplyHandle, ProbeTouch};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeManifest {
    pub probe_id: String,
    pub loop_id: String,
    pub touched: Vec<ProbeTouch>,
    pub revert_metadata: serde_json::Value,
}

impl ProbeManifest {
    pub fn from_apply(handle: &ApplyHandle, loop_id: impl Into<String>) -> Self {
        Self {
            probe_id: handle.probe_id.clone(),
            loop_id: loop_id.into(),
            touched: handle.touched.clone(),
            revert_metadata: handle.revert_metadata.clone(),
        }
    }

    /// Emit this manifest as an `audit_ledger::LedgerEntry`.
    pub fn into_ledger_entry(self) -> LedgerEntry {
        LedgerEntry::new("probe.apply")
            .with_loop_id(self.loop_id.clone())
            .with_action(self.probe_id.clone())
            .with_evidence(serde_json::json!({
                "probe_id": self.probe_id,
                "loop_id": self.loop_id,
                "touched": self.touched,
                "revert_metadata": self.revert_metadata,
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn manifest_serializes() {
        let m = ProbeManifest {
            probe_id: "p1".into(),
            loop_id: "L1".into(),
            touched: vec![ProbeTouch::EnvVar("X".into())],
            revert_metadata: serde_json::json!({"prior": null}),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ProbeManifest = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn manifest_loop_shadow_touch_serializes() {
        let m = ProbeManifest {
            probe_id: "p2".into(),
            loop_id: "L2".into(),
            touched: vec![ProbeTouch::LoopShadowFile(PathBuf::from(
                "/tmp/loop/cwd-shadow/a.txt",
            ))],
            revert_metadata: serde_json::json!({"abs_path": "/tmp/loop/cwd-shadow/a.txt"}),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ProbeManifest = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn ledger_entry_kind_is_probe_apply() {
        let h = ApplyHandle {
            probe_id: "p3".into(),
            touched: vec![ProbeTouch::EnvVar("Y".into())],
            revert_metadata: serde_json::json!({}),
        };
        let m = ProbeManifest::from_apply(&h, "L3");
        let entry = m.into_ledger_entry();
        assert_eq!(entry.kind, "probe.apply");
        assert_eq!(entry.loop_id.as_deref(), Some("L3"));
        assert_eq!(entry.action.as_deref(), Some("p3"));
    }
}

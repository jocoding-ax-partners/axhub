//! `EnvVarProbe` — snapshots an env var, sets it to a new value, restores on
//! revert. Plan v6 §4.4 — v0.8.0 builtin probe.

use serde_json::json;

use super::super::DiagnoseError;
use super::{ApplyHandle, Probe, ProbeContext, ProbeTouch};

pub struct EnvVarProbe {
    pub id: String,
    pub hypothesis_id: String,
    pub var_name: String,
    pub new_value: Option<String>,
}

impl Probe for EnvVarProbe {
    fn id(&self) -> &str {
        &self.id
    }
    fn hypothesis_id(&self) -> &str {
        &self.hypothesis_id
    }
    fn touches(&self) -> Vec<ProbeTouch> {
        vec![ProbeTouch::EnvVar(self.var_name.clone())]
    }
    fn apply(&self, _ctx: &ProbeContext) -> Result<ApplyHandle, DiagnoseError> {
        // Use the crate-wide PROCESS_ENV_LOCK so this probe serialises with
        // every other env-mutating site (recurrence threshold overrides,
        // preflight wall-budget overrides, consent headless guards, tests).
        // Recover from poisoning instead of panicking — fail-open hook
        // contract (CLAUDE.md axhub Hook Safety §10.6).
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        // Manifest distinguishes "prior unset" (None → JSON null) from
        // "prior was an empty string" (Some(String::new()) → "") so revert
        // can restore each case correctly.
        let prior_kind = match std::env::var(&self.var_name) {
            Ok(v) => json!({ "kind": "set", "value": v }),
            Err(_) => json!({ "kind": "unset" }),
        };
        match &self.new_value {
            Some(v) => std::env::set_var(&self.var_name, v),
            None => std::env::remove_var(&self.var_name),
        }
        Ok(ApplyHandle {
            probe_id: self.id.clone(),
            touched: self.touches(),
            revert_metadata: json!({ "prior": prior_kind }),
        })
    }
    fn revert(&self, handle: ApplyHandle) -> Result<(), DiagnoseError> {
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let prior = handle.revert_metadata.get("prior").ok_or_else(|| {
            DiagnoseError::CleanupFailed(format!(
                "EnvVarProbe revert manifest missing `prior` for var={}",
                self.var_name
            ))
        })?;
        let kind = prior.get("kind").and_then(|v| v.as_str()).ok_or_else(|| {
            DiagnoseError::CleanupFailed(format!(
                "EnvVarProbe revert manifest missing `prior.kind` for var={}",
                self.var_name
            ))
        })?;
        match kind {
            "set" => {
                let value = prior.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
                    DiagnoseError::CleanupFailed(format!(
                        "EnvVarProbe revert manifest `prior.kind=set` missing `value` for var={}",
                        self.var_name
                    ))
                })?;
                std::env::set_var(&self.var_name, value);
            }
            "unset" => std::env::remove_var(&self.var_name),
            other => {
                return Err(DiagnoseError::CleanupFailed(format!(
                    "EnvVarProbe revert manifest unknown `prior.kind={other}` for var={}",
                    self.var_name
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_and_reverts_env_var() {
        let var = "AXHUB_TEST_ENV_VAR_PROBE_VAR_A";
        std::env::remove_var(var); // baseline
        let p = EnvVarProbe {
            id: "p1".into(),
            hypothesis_id: "H1".into(),
            var_name: var.into(),
            new_value: Some("hello".into()),
        };
        let ctx = ProbeContext {
            loop_id: "loop-t".into(),
            shadow_root: std::path::PathBuf::from("/tmp/shadow"),
        };
        let handle = p.apply(&ctx).unwrap();
        assert_eq!(std::env::var(var).unwrap(), "hello");
        p.revert(handle).unwrap();
        assert!(std::env::var(var).is_err(), "var must be unset on revert");
    }

    #[test]
    fn restores_prior_value() {
        let var = "AXHUB_TEST_ENV_VAR_PROBE_VAR_B";
        std::env::set_var(var, "original");
        let p = EnvVarProbe {
            id: "p2".into(),
            hypothesis_id: "H2".into(),
            var_name: var.into(),
            new_value: Some("new".into()),
        };
        let ctx = ProbeContext {
            loop_id: "loop-t".into(),
            shadow_root: std::path::PathBuf::from("/tmp/shadow"),
        };
        let handle = p.apply(&ctx).unwrap();
        assert_eq!(std::env::var(var).unwrap(), "new");
        p.revert(handle).unwrap();
        assert_eq!(
            std::env::var(var).unwrap(),
            "original",
            "must restore prior value"
        );
        std::env::remove_var(var);
    }

    #[test]
    fn remove_value_variant() {
        let var = "AXHUB_TEST_ENV_VAR_PROBE_VAR_C";
        std::env::set_var(var, "existing");
        let p = EnvVarProbe {
            id: "p3".into(),
            hypothesis_id: "H3".into(),
            var_name: var.into(),
            new_value: None,
        };
        let ctx = ProbeContext {
            loop_id: "loop-t".into(),
            shadow_root: std::path::PathBuf::from("/tmp/shadow"),
        };
        let handle = p.apply(&ctx).unwrap();
        assert!(std::env::var(var).is_err());
        p.revert(handle).unwrap();
        assert_eq!(std::env::var(var).unwrap(), "existing");
        std::env::remove_var(var);
    }

    #[test]
    fn touches_returns_env_var() {
        let p = EnvVarProbe {
            id: "p4".into(),
            hypothesis_id: "H4".into(),
            var_name: "SOME_VAR".into(),
            new_value: None,
        };
        let touches = p.touches();
        assert_eq!(touches.len(), 1);
        assert_eq!(touches[0], ProbeTouch::EnvVar("SOME_VAR".into()));
    }
}

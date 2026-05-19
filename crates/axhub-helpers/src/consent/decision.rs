//! 4-variant consent decision wrapper for the auto-diagnose system.
//!
//! Layers on top of the existing `crate::consent::jwt` HMAC-JWT infrastructure
//! to encode user intent at AskUserQuestion time:
//! - `Once`: allow this single action (TTL 60s)
//! - `AllowSession`: allow within current session (TTL 1h)
//! - `AllowAlways`: allow always (TTL 1y), session_id="unknown" rejected
//! - `Deny`: refuse
//!
//! See plan v6 §13.B (Phase 0a) and phase-1c-consent.md.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::jwt::{mint_token, verify_token, ConsentBinding, MintResult, VerifyResult};
use super::key::session_id;

/// TTL (seconds) for each variant.
pub const TTL_ONCE_SEC: i64 = 60;
pub const TTL_SESSION_SEC: i64 = 60 * 60;
pub const TTL_ALWAYS_SEC: i64 = 60 * 60 * 24 * 365;

/// User decision variants emitted by AskUserQuestion at diagnose-loop entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionVariant {
    Once,
    AllowSession,
    AllowAlways,
    Deny,
}

#[derive(Debug, Error)]
pub enum DecisionError {
    /// session_id resolves to "unknown" (no `CLAUDE_SESSION_ID` env).
    /// Long-lived grants (`AllowSession`/`AllowAlways`) are rejected to
    /// prevent profile poisoning in headless / CI environments.
    #[error("session_id is 'unknown' — long-TTL grants rejected (headless env)")]
    HeadlessEnvironment,

    #[error("HMAC mint failed: {0}")]
    MintFailed(#[source] anyhow::Error),

    #[error("session_id resolution failed: {0}")]
    SessionIdLookup(#[source] anyhow::Error),
}

/// Issue a decision token for the chosen variant. Returns `Ok(None)` for `Deny`.
///
/// For `AllowSession` and `AllowAlways`, fails with `HeadlessEnvironment` if
/// the current process has no resolvable session_id.
pub fn issue_decision_token(
    variant: DecisionVariant,
    binding: ConsentBinding,
) -> Result<Option<MintResult>, DecisionError> {
    match variant {
        DecisionVariant::Deny => Ok(None),
        DecisionVariant::Once => mint_token(binding, TTL_ONCE_SEC)
            .map(Some)
            .map_err(DecisionError::MintFailed),
        DecisionVariant::AllowSession | DecisionVariant::AllowAlways => {
            let sid = match session_id() {
                Ok(s) => s,
                Err(_) => return Err(DecisionError::HeadlessEnvironment),
            };
            if sid.is_empty() || sid == "unknown" {
                return Err(DecisionError::HeadlessEnvironment);
            }
            let ttl = if variant == DecisionVariant::AllowSession {
                TTL_SESSION_SEC
            } else {
                TTL_ALWAYS_SEC
            };
            mint_token(binding, ttl)
                .map(Some)
                .map_err(DecisionError::MintFailed)
        }
    }
}

/// Verify an existing decision token bound to `binding`.
/// Returns the underlying JWT verification result (TTL, signature, binding match).
pub fn check_decision(binding: ConsentBinding) -> VerifyResult {
    verify_token(binding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn binding() -> ConsentBinding {
        ConsentBinding {
            tool_call_id: "test-tc-001".into(),
            action: "diagnose".into(),
            app_id: "axhub".into(),
            profile: "default".into(),
            branch: "main".into(),
            commit_sha: "abc1234".into(),
            context: HashMap::new(),
            synthesized_by_helper: false,
        }
    }

    #[test]
    fn deny_returns_none() {
        let _g = ENV_LOCK.lock().unwrap();
        let result = issue_decision_token(DecisionVariant::Deny, binding()).unwrap();
        assert!(result.is_none(), "Deny must return None");
    }

    #[test]
    fn once_short_ttl() {
        let _g = ENV_LOCK.lock().unwrap();
        assert_eq!(TTL_ONCE_SEC, 60);
    }

    #[test]
    fn ttl_ordering() {
        assert!(TTL_ONCE_SEC < TTL_SESSION_SEC);
        assert!(TTL_SESSION_SEC < TTL_ALWAYS_SEC);
    }

    #[test]
    fn allow_session_rejects_unknown_session() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("CLAUDE_SESSION_ID", "unknown");
        let result = issue_decision_token(DecisionVariant::AllowSession, binding());
        std::env::remove_var("CLAUDE_SESSION_ID");
        assert!(matches!(result, Err(DecisionError::HeadlessEnvironment)));
    }

    #[test]
    fn allow_always_rejects_empty_session() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("CLAUDE_SESSION_ID");
        let result = issue_decision_token(DecisionVariant::AllowAlways, binding());
        assert!(matches!(result, Err(DecisionError::HeadlessEnvironment)));
    }
}

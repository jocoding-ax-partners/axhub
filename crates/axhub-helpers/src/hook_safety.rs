// Phase 25 PR 25.2 — Hook safety primitive (kill switch + fail-open helper).
//
// Provides `is_hook_disabled(hook_name)` so each axhub hook entry point can
// short-circuit when the user opts out. Two canonical env vars per
// `.plan/matrix-absorption/00-overview.md` §10.6 (Env Var Taxonomy ADR):
//
//   AXHUB_DISABLE_HOOKS=1                → all axhub hooks disabled
//   AXHUB_DISABLE_HOOK=<name>,<name>     → specific hooks disabled (csv)
//
// Legacy `DISABLE_AXHUB=1` is honored as an alias for the 6-month deprecation
// window (planned removal at v0.8.0) and emits a one-shot stderr warning so
// users notice the rename.
//
// All hooks MUST fail open. When a hook implementation panics or returns a
// non-recoverable error the caller surfaces a `systemMessage` to the user and
// returns exit 0. `append_hook_error(name, err)` writes a structured line to
// `$XDG_STATE_HOME/axhub-plugin/hook-errors.jsonl` so we can audit failures
// without breaking the user's main flow.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::runtime_paths::state_dir;

/// Canonical env: disable every axhub hook entry point.
pub const ENV_DISABLE_ALL: &str = "AXHUB_DISABLE_HOOKS";
/// Canonical env: disable specific axhub hook entry points (csv).
pub const ENV_DISABLE_LIST: &str = "AXHUB_DISABLE_HOOK";
/// Legacy alias retained for the 6-month deprecation window (overview §10.6).
pub const LEGACY_DISABLE_ALL: &str = "DISABLE_AXHUB";
/// ADR-0012 — per-feature opt-out for statusLine autowire (no legacy alias).
pub const ENV_DISABLE_STATUSLINE_AUTOWIRE: &str = "AXHUB_DISABLE_STATUSLINE_AUTOWIRE";
/// Phase 26 env: disable quality next-turn triggers and commit gate review prompts.
pub const ENV_DISABLE_TRIGGERS: &str = "AXHUB_DISABLE_TRIGGERS";
/// Phase 26 env: disable megaskill SessionStart context.
pub const ENV_DISABLE_MEGASKILL: &str = "AXHUB_DISABLE_MEGASKILL";
/// Phase 26 env: disable Karpathy UserPromptSubmit context.
pub const ENV_DISABLE_KARPATHY: &str = "AXHUB_DISABLE_KARPATHY";
/// Phase 26 env: disable optional post-commit state promotion.
pub const ENV_DISABLE_POSTCOMMIT: &str = "AXHUB_DISABLE_POSTCOMMIT";

static LEGACY_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

/// Returns true when the named hook MUST short-circuit (caller writes
/// `{"continue":true}` or `exit 0` and returns).
///
/// Precedence:
///   1. canonical `AXHUB_DISABLE_HOOKS=1` (global)
///   2. canonical `AXHUB_DISABLE_HOOK=<csv>` (per-hook)
///   3. legacy `DISABLE_AXHUB=1` alias (warns once, retire at v0.8.0)
pub fn is_hook_disabled(hook_name: &str) -> bool {
    if env_truthy(ENV_DISABLE_ALL) {
        return true;
    }
    if let Ok(raw) = std::env::var(ENV_DISABLE_LIST) {
        if csv_contains(&raw, hook_name) {
            return true;
        }
    }
    if env_truthy(LEGACY_DISABLE_ALL) {
        emit_legacy_warning_once();
        return true;
    }
    false
}

/// Returns true when `AXHUB_DISABLE_STATUSLINE_AUTOWIRE` is set to a truthy
/// value (1/true/yes/on). No legacy alias per ADR-0012 §10.6 polarity rules.
pub fn is_statusline_autowire_disabled() -> bool {
    env_truthy(ENV_DISABLE_STATUSLINE_AUTOWIRE)
}

/// Returns true when quality reminder / commit-gate triggers are disabled.
pub fn is_quality_triggers_disabled() -> bool {
    env_truthy(ENV_DISABLE_TRIGGERS)
}

/// Returns true when the SessionStart megaskill context is disabled.
pub fn is_megaskill_disabled() -> bool {
    env_truthy(ENV_DISABLE_MEGASKILL) || is_quality_triggers_disabled()
}

/// Returns true when the Karpathy coding reminder context is disabled.
pub fn is_karpathy_disabled() -> bool {
    env_truthy(ENV_DISABLE_KARPATHY) || is_quality_triggers_disabled()
}

/// Returns true when the optional post-commit promotion hook is disabled.
pub fn is_postcommit_disabled() -> bool {
    env_truthy(ENV_DISABLE_POSTCOMMIT) || is_quality_triggers_disabled()
}

/// Append a structured failure line for post-mortem inspection. Best-effort —
/// any IO error is swallowed so callers never raise on bookkeeping issues.
pub fn append_hook_error(hook_name: &str, err: &dyn std::fmt::Display) {
    let Some(dir) = state_dir() else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("hook-errors.jsonl");
    let line = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "hook": hook_name,
        "error": err.to_string(),
    })
    .to_string();

    let mut opts = OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    if let Ok(mut file) = opts.open(&path) {
        let _ = writeln!(file, "{}", line);
        let _ = file.sync_all();
    }
}

fn env_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

fn csv_contains(raw: &str, needle: &str) -> bool {
    raw.split(',').any(|item| item.trim() == needle)
}

fn emit_legacy_warning_once() {
    if LEGACY_WARNING_EMITTED.swap(true, Ordering::SeqCst) {
        return;
    }
    eprintln!(
        "[axhub] warning: `{legacy}` 는 deprecated 됐어요. v0.8.0 에서 제거 예정 — \
canonical 한 `{canonical}` 또는 `{per_hook}=<csv>` 로 옮겨주세요.",
        legacy = LEGACY_DISABLE_ALL,
        canonical = ENV_DISABLE_ALL,
        per_hook = ENV_DISABLE_LIST,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests in this module mutate process env, so serialize them with all other
    // env-mutating tests via the process-wide lock.
    use crate::PROCESS_ENV_LOCK;
    macro_rules! env_lock {
        () => {
            PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
        };
    }

    fn reset_legacy_flag() {
        LEGACY_WARNING_EMITTED.store(false, Ordering::SeqCst);
    }

    fn clear_envs() {
        std::env::remove_var(ENV_DISABLE_ALL);
        std::env::remove_var(ENV_DISABLE_LIST);
        std::env::remove_var(LEGACY_DISABLE_ALL);
        std::env::remove_var(ENV_DISABLE_TRIGGERS);
        std::env::remove_var(ENV_DISABLE_MEGASKILL);
        std::env::remove_var(ENV_DISABLE_KARPATHY);
        std::env::remove_var(ENV_DISABLE_POSTCOMMIT);
    }

    #[test]
    fn default_state_does_not_disable() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        assert!(!is_hook_disabled("session-start"));
    }

    #[test]
    fn global_env_disables_every_hook() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        std::env::set_var(ENV_DISABLE_ALL, "1");
        assert!(is_hook_disabled("session-start"));
        assert!(is_hook_disabled("classify-exit"));
        clear_envs();
    }

    #[test]
    fn per_hook_env_disables_only_listed() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        std::env::set_var(ENV_DISABLE_LIST, "session-start , preauth-check");
        assert!(is_hook_disabled("session-start"));
        assert!(is_hook_disabled("preauth-check"));
        assert!(!is_hook_disabled("classify-exit"));
        clear_envs();
    }

    #[test]
    fn legacy_alias_disables_with_warning() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        std::env::set_var(LEGACY_DISABLE_ALL, "1");
        assert!(is_hook_disabled("session-start"));
        clear_envs();
    }

    #[test]
    fn truthy_variants_recognized() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        for value in ["1", "true", "yes", "on"] {
            std::env::set_var(ENV_DISABLE_ALL, value);
            assert!(is_hook_disabled("any"), "value `{value}` should be truthy");
        }
        clear_envs();
    }

    #[test]
    fn falsy_or_unset_does_not_disable() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        for value in ["0", "false", "no", ""] {
            std::env::set_var(ENV_DISABLE_ALL, value);
            assert!(
                !is_hook_disabled("any"),
                "value `{value}` should not disable"
            );
        }
        clear_envs();
    }
    #[test]
    fn quality_trigger_env_helpers_follow_truthy_contract() {
        let _guard = env_lock!();
        clear_envs();
        reset_legacy_flag();
        std::env::set_var(ENV_DISABLE_TRIGGERS, "1");
        assert!(is_quality_triggers_disabled());
        assert!(is_megaskill_disabled());
        assert!(is_karpathy_disabled());
        assert!(is_postcommit_disabled());
        clear_envs();
    }
}

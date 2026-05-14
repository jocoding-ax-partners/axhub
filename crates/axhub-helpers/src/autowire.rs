//! v0.6.0 — SessionStart statusLine autowire.
//!
//! `autowire_statusline()` is the entry point for `axhub-helpers autowire-statusline`.
//!
//! Decision flow:
//!   0. Kill-switch: `AXHUB_DISABLE_HOOKS` / `AXHUB_DISABLE_HOOK=session-start-autowire`
//!      / `AXHUB_DISABLE_STATUSLINE_AUTOWIRE` → exit 0
//!   1. Disclosure marker absent → emit disclosure via stderr + write marker + exit 0
//!      (first session shows disclosure; merge starts on next session — ADR-0012)
//!   2. Marker mtime within 60 s (subprocess race guard) → exit 0
//!   3. Call orphan_stub::install_and_verify → get stub path
//!   4. Call settings_merge::merge with stub path override → get outcome
//!   5. Write scope marker (dispatcher only, never child subprocess)
//!   6. Append observability event

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::hook_safety::{append_hook_error, is_hook_disabled, is_statusline_autowire_disabled};
use crate::observability::append_autowire_event;
use crate::orphan_stub;
use crate::runtime_paths::state_dir;
use crate::settings_merge::{merge, MergeOptions, MergeOutcome, Scope};

/// Subprocess race-guard window: skip if scope marker is newer than this.
const MARKER_SKIP_SECS: u64 = 60;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Arguments for the `autowire-statusline` subcommand.
#[derive(Debug)]
pub struct AutowireArgs {
    /// Target scope (user / project). Auto not accepted here — caller must resolve.
    pub scope: Scope,
    /// If Some, override the command path written to settings.json.
    /// If None, the orphan stub path is installed + used automatically.
    pub command_path_override: Option<PathBuf>,
    /// Suppress human-readable stderr messages (hook caller mode).
    pub silent: bool,
    /// If true, write scope marker after merge (dispatcher = true; child claude -p = false).
    pub is_dispatcher: bool,
}

/// Run the full autowire pipeline. Always returns `Ok(0)` (fail-open).
pub fn autowire_statusline(args: AutowireArgs) -> i32 {
    match run_autowire(args) {
        Ok(code) => code,
        Err(e) => {
            append_hook_error("session-start-autowire", &e);
            0 // fail-open
        }
    }
}

// ---------------------------------------------------------------------------
// Internal pipeline
// ---------------------------------------------------------------------------

fn run_autowire(args: AutowireArgs) -> anyhow::Result<i32> {
    // --- Kill switches ---
    if is_hook_disabled("session-start-autowire") {
        return Ok(0);
    }
    if is_statusline_autowire_disabled() {
        return Ok(0);
    }

    let Some(sd) = state_dir() else {
        return Ok(0); // cannot resolve state dir — skip silently
    };

    // --- Disclosure marker check ---
    let disclosure_marker = sd.join("install-disclosure-shown.txt");
    if !disclosure_marker.exists() {
        emit_disclosure_message();
        write_disclosure_marker(&disclosure_marker);
        // First encounter: show disclosure, skip merge this session (ADR-0012 §B step 0c)
        return Ok(0);
    }

    // --- Scope marker mtime check (subprocess race guard, S5) ---
    let scope_label = scope_label(&args.scope);
    let marker_path = sd.join(format!("auto-wire-done-{scope_label}.json"));
    if within_dedup_window(&marker_path) {
        return Ok(0);
    }

    // --- Orphan stub: install + verify ---
    let command_path = match args.command_path_override {
        Some(p) => p,
        None => match orphan_stub::install_and_verify() {
            Some(p) => p,
            None => {
                // Stub install/verify failed — skip merge (settings.json must not
                // point at a broken stub path).
                append_hook_error(
                    "session-start-autowire",
                    &"orphan stub install/verify 실패 — merge 건너뛰었어요",
                );
                return Ok(0);
            }
        },
    };

    // --- Foundation merge ---
    let scope_for_obs = scope_label.to_string();
    let opts = MergeOptions {
        silent: args.silent,
        command_path_override: Some(command_path),
        scope: args.scope,
        dry_run: false,
    };

    let outcome = match merge(opts) {
        Ok(o) => o,
        Err(e) => {
            append_hook_error("session-start-autowire", &e);
            return Ok(0); // fail-open
        }
    };

    // --- Write scope marker (dispatcher only) ---
    if args.is_dispatcher {
        write_scope_marker(&marker_path);
    }

    // --- Observability ---
    let other_cmd = match &outcome {
        MergeOutcome::PreservedOther => {
            // We don't have the other plugin's command string here (it's inside
            // the foundation). Pass None; FU-4 will thread it through.
            None
        }
        _ => None,
    };
    if let Err(e) = append_autowire_event(&outcome, &scope_for_obs, other_cmd) {
        append_hook_error("session-start-autowire", &e);
    }

    Ok(0) // always exit 0 — fail-open hook contract
}

// ---------------------------------------------------------------------------
// TTY detection (plan §B TTY detect algorithm)
// ---------------------------------------------------------------------------

/// Returns `true` when running in a non-interactive context (CI / `-p` / no TTY).
pub fn is_non_interactive() -> bool {
    if std::env::var("CLAUDE_NON_INTERACTIVE").as_deref() == Ok("1") {
        return true;
    }
    if std::env::var("CI").is_ok() {
        return true;
    }
    if std::env::var("CLAUDE_NO_TTY").as_deref() == Ok("1") {
        return true;
    }
    // Check parent TTY hint (set by Claude Code in some contexts).
    if std::env::var("CLAUDE_PARENT_TTY").as_deref() == Ok("0") {
        return true;
    }
    // stdout isatty check — in hook subprocess context this is usually false.
    // Fall back to "interactive" if we can't determine (avoid blocking CI).
    #[cfg(unix)]
    {
        // SAFETY: isatty(1) is always safe to call.
        let rc = unsafe { libc::isatty(1) };
        if rc == 0 {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Disclosure emission
// ---------------------------------------------------------------------------

fn emit_disclosure_message() {
    // Use systemMessage JSON format so Claude Code renders it visibly.
    // In non-interactive contexts stderr still captures it for logs.
    let msg = r#"axhub 이 다음을 수행해요:
  (1) 인증 토큰을 keychain (macOS/Windows) / file (Linux) 에 저장해요.
  (2) opt-in telemetry 가 활성화되어 있어요 (AXHUB_TELEMETRY=0 로 disable).
  (3) macOS Gatekeeper 의 helper binary quarantine attribute 를 제거해요.
  (4) auth-refresh 백그라운드 task 가 token 갱신해요.
  (5) helper binary 를 GitHub release 에서 HTTPS 로 다운로드 + 실행해요.
  (6) ~/.claude/settings.json 의 statusLine field 를 추가/관리해요 (other plugins preserved).

거부하려면 AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1 후 재시작해주세요. uninstall 시 orphan stub 가 자동 fallback 해요."#;

    // Emit as JSON systemMessage (Claude Code hook format).
    let json = serde_json::json!({ "systemMessage": msg });
    println!("{json}");
}

fn write_disclosure_marker(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = format!(
        "{{\"shown_at\":\"{}\"}}\n",
        chrono::Utc::now().to_rfc3339()
    );
    let _ = fs::write(path, content);
}

// ---------------------------------------------------------------------------
// Scope marker helpers
// ---------------------------------------------------------------------------

fn scope_label(scope: &Scope) -> &'static str {
    match scope {
        Scope::User => "user",
        Scope::Project => "project",
        Scope::Auto => "auto",
    }
}

fn within_dedup_window(marker_path: &std::path::Path) -> bool {
    let Ok(meta) = fs::metadata(marker_path) else {
        return false;
    };
    let Ok(mtime) = meta.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(mtime) else {
        return false;
    };
    elapsed < Duration::from_secs(MARKER_SKIP_SECS)
}

fn write_scope_marker(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = format!(
        "{{\"wired_at\":\"{}\"}}\n",
        chrono::Utc::now().to_rfc3339()
    );
    let _ = fs::write(path, content);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! env_lock {
        () => {
            crate::PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
        };
    }

    fn clear_kill_switches() {
        unsafe {
            std::env::remove_var("AXHUB_DISABLE_HOOKS");
            std::env::remove_var("AXHUB_DISABLE_HOOK");
            std::env::remove_var("AXHUB_DISABLE_STATUSLINE_AUTOWIRE");
            std::env::remove_var("DISABLE_AXHUB");
        }
    }

    #[test]
    fn scope_label_all_variants() {
        assert_eq!(scope_label(&Scope::User), "user");
        assert_eq!(scope_label(&Scope::Project), "project");
        assert_eq!(scope_label(&Scope::Auto), "auto");
    }

    #[test]
    fn within_dedup_window_false_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nonexistent.json");
        assert!(!within_dedup_window(&missing));
    }

    #[test]
    fn within_dedup_window_true_for_fresh_file() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker.json");
        fs::write(&marker, b"{}").unwrap();
        assert!(within_dedup_window(&marker), "fresh marker should trigger skip");
    }

    #[test]
    fn is_non_interactive_ci_env() {
        let _guard = env_lock!();
        unsafe {
            std::env::remove_var("CLAUDE_NON_INTERACTIVE");
            std::env::remove_var("CLAUDE_NO_TTY");
            std::env::remove_var("CLAUDE_PARENT_TTY");
            std::env::set_var("CI", "true");
        }
        assert!(is_non_interactive(), "CI env should be non-interactive");
        unsafe {
            std::env::remove_var("CI");
        }
    }

    #[test]
    fn is_non_interactive_claude_non_interactive_env() {
        let _guard = env_lock!();
        unsafe {
            std::env::remove_var("CI");
            std::env::remove_var("CLAUDE_NO_TTY");
            std::env::remove_var("CLAUDE_PARENT_TTY");
            std::env::set_var("CLAUDE_NON_INTERACTIVE", "1");
        }
        assert!(is_non_interactive());
        unsafe {
            std::env::remove_var("CLAUDE_NON_INTERACTIVE");
        }
    }

    #[test]
    fn is_non_interactive_claude_no_tty_env() {
        let _guard = env_lock!();
        unsafe {
            std::env::remove_var("CI");
            std::env::remove_var("CLAUDE_NON_INTERACTIVE");
            std::env::remove_var("CLAUDE_PARENT_TTY");
            std::env::set_var("CLAUDE_NO_TTY", "1");
        }
        assert!(is_non_interactive());
        unsafe {
            std::env::remove_var("CLAUDE_NO_TTY");
        }
    }

    #[test]
    fn autowire_skips_when_global_hook_disabled() {
        let _guard = env_lock!();
        clear_kill_switches();
        unsafe {
            std::env::set_var("AXHUB_DISABLE_HOOKS", "1");
        }
        let code = autowire_statusline(AutowireArgs {
            scope: Scope::User,
            command_path_override: None,
            silent: true,
            is_dispatcher: false,
        });
        assert_eq!(code, 0);
        clear_kill_switches();
    }

    #[test]
    fn autowire_skips_when_statusline_autowire_disabled() {
        let _guard = env_lock!();
        clear_kill_switches();
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("XDG_STATE_HOME", dir.path());
            std::env::set_var("AXHUB_DISABLE_STATUSLINE_AUTOWIRE", "1");
        }
        let code = autowire_statusline(AutowireArgs {
            scope: Scope::User,
            command_path_override: None,
            silent: true,
            is_dispatcher: false,
        });
        assert_eq!(code, 0);
        clear_kill_switches();
        unsafe {
            std::env::remove_var("XDG_STATE_HOME");
        }
    }

    struct EnvGuard {
        vars: Vec<(String, Option<std::ffi::OsString>)>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        fn new(lock: std::sync::MutexGuard<'static, ()>, keys: &[&str]) -> Self {
            let vars = keys
                .iter()
                .map(|k| (k.to_string(), std::env::var_os(k)))
                .collect();
            Self { vars, _lock: lock }
        }

        fn set(&self, key: &str, val: &std::ffi::OsStr) {
            unsafe { std::env::set_var(key, val) }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, old) in &self.vars {
                match old {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[test]
    fn marker_written_after_dedup_window_for_dispatcher() {
        let guard = EnvGuard::new(
            env_lock!(),
            &[
                "XDG_STATE_HOME",
                "HOME",
                "USERPROFILE",
                "AXHUB_DISABLE_HOOKS",
                "AXHUB_DISABLE_HOOK",
                "AXHUB_DISABLE_STATUSLINE_AUTOWIRE",
                "DISABLE_AXHUB",
            ],
        );

        let state_dir_tmp = tempfile::tempdir().unwrap();
        let home_tmp = tempfile::tempdir().unwrap();
        guard.set("XDG_STATE_HOME", state_dir_tmp.path().as_os_str());
        guard.set("HOME", home_tmp.path().as_os_str());
        guard.set("USERPROFILE", home_tmp.path().as_os_str());
        clear_kill_switches();

        // Pre-write disclosure marker so we skip straight to merge
        let sd = state_dir_tmp.path().join("axhub-plugin");
        fs::create_dir_all(&sd).unwrap();
        fs::write(sd.join("install-disclosure-shown.txt"), b"{}").unwrap();

        // Provide a dummy command path (avoid real orphan stub install for this test)
        let stub_path = sd.join("dummy-stub.sh");
        fs::write(&stub_path, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&stub_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&stub_path, perms).unwrap();
        }

        let code = autowire_statusline(AutowireArgs {
            scope: Scope::User,
            command_path_override: Some(stub_path),
            silent: true,
            is_dispatcher: true,
        });
        assert_eq!(code, 0, "autowire should always return 0");

        // Scope marker should be written by dispatcher
        let marker = sd.join("auto-wire-done-user.json");
        assert!(marker.exists(), "scope marker should exist after dispatcher run");

        // EnvGuard Drop restores all env vars automatically.
        drop(guard);
    }
}

//! Proactive **CLI binary** version-drift nudge (plan: docs/plans/plugin-update-nudge.md).
//!
//! Sibling of `plugin_update.rs`. The plugin channel watches the plugin's own
//! GitHub releases (ureq); this channel watches the separately-versioned `axhub`
//! CLI via its authoritative `axhub update check --json` contract. That command
//! returns a backend-computed `has_update` plus the CLI's installed `current`,
//! so this channel uses `has_update` directly rather than recomputing a local
//! semver gate (the plugin's `is_newer`/`CARGO_PKG_VERSION` baseline is wrong
//! for the CLI — they version independently).
//!
//! Data flow — every path is fail-open (any error → no nudge, exit 0):
//!
//! ```text
//!   cli-latest-fetch-bg  (spawned detached from session-start.sh, axhub-guarded)
//!     axhub update check --json ──▶ parse {current,latest,has_update,disabled} ──▶ atomic cache (TTL 24h)
//!        │                            │                                              │
//!        ▼                            ▼                                              ▼
//!     [timed_out] → skip          [malformed/empty] → skip                       [io fail] → skip
//!
//!   prompt-route  (UserPromptSubmit — the reliable steering surface)
//!     read cache ──▶ backend has_update ──▶ Some(nudge) + per-version marker
//!        │              │                     │
//!        ▼              ▼                     ▼
//!     [absent/stale] [disabled / !has_update] [marker/optout/non-interactive] → None
//! ```

use std::fs;

use serde::{Deserialize, Serialize};

use crate::axhub_cli::run_axhub;
use crate::hook_safety::is_hook_disabled;
use crate::plugin_update::{is_newer, is_non_interactive, normalize_tag, now_unix};
use crate::runtime_paths::{
    cli_drift_nudge_marker_path, cli_drift_optout_path, cli_latest_cache_path,
};

const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// Cached result of the most recent `axhub update check --json`. Unlike the
/// plugin's `LatestCache`, this stores `current` (the CLI's installed version,
/// which differs from the plugin's `CARGO_PKG_VERSION`) plus the backend's
/// authoritative `has_update`/`disabled` flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliLatestCache {
    /// CLI installed version, normalized (no leading `v`).
    pub current: String,
    /// Latest available CLI version, normalized (no leading `v`). Empty when disabled.
    pub latest: String,
    /// Backend-computed update availability (authoritative — not recomputed locally).
    pub has_update: bool,
    /// `true` when the CLI's own auto-update check is disabled
    /// (`AXHUB_DISABLE_AUTOUPDATE`). Suppresses the nudge entirely.
    pub disabled: bool,
    /// Unix seconds at which the check succeeded.
    pub fetched_at: u64,
}

fn optout_present() -> bool {
    cli_drift_optout_path().map(|p| p.exists()).unwrap_or(false)
}

fn read_cache() -> Option<CliLatestCache> {
    let path = cli_latest_cache_path()?;
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Atomic cache write: serialize to a sibling temp file then rename, so a
/// concurrent reader never sees a torn file.
fn write_cache(cache: &CliLatestCache) -> std::io::Result<()> {
    let path = cli_latest_cache_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no cache path"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Parse `axhub update check --json`. Returns `None` on empty/malformed output or
/// a missing `current` field (fail-open: never nudge on garbage). `current`/
/// `latest` are normalized (the CLI emits a `v` prefix, e.g. `v0.18.1`).
fn parse_cli_update_check(stdout: &str) -> Option<CliLatestCache> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let current = normalize_tag(value.get("current")?.as_str()?);
    if current.is_empty() {
        return None;
    }
    let latest = value
        .get("latest")
        .and_then(serde_json::Value::as_str)
        .map(normalize_tag)
        .unwrap_or_default();
    let has_update = value
        .get("has_update")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let disabled = value
        .get("disabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    Some(CliLatestCache {
        current,
        latest,
        has_update,
        disabled,
        fetched_at: now_unix(),
    })
}

/// Run `axhub update check --json` and parse it. `None` when the CLI is absent,
/// times out, or emits unparseable output — all fail-open.
fn fetch_cli_update_check() -> Option<CliLatestCache> {
    let out = run_axhub(&["update", "check", "--json"]);
    if out.timed_out {
        return None;
    }
    parse_cli_update_check(&out.stdout)
}

/// Pure drift decision — no IO, so every branch is unit-testable. Returns `true`
/// when the CLI nudge should fire this turn. Uses the backend `has_update` flag
/// directly (no local semver recompute).
fn cli_should_nudge(
    cache: Option<&CliLatestCache>,
    now: u64,
    marker_exists: bool,
    optout: bool,
    non_interactive: bool,
) -> bool {
    if optout || non_interactive {
        return false;
    }
    let Some(cache) = cache else {
        return false;
    };
    if cache.disabled {
        return false; // CLI auto-update disabled (AXHUB_DISABLE_AUTOUPDATE)
    }
    if now.saturating_sub(cache.fetched_at) >= CACHE_TTL_SECS {
        return false; // stale cache
    }
    if !cache.has_update {
        return false; // backend says current (authoritative)
    }
    // Defense in depth: `has_update` is the backend's authoritative signal for
    // *whether* an update exists, but we still require `latest` to be a
    // well-formed version strictly newer than the CLI's OWN `current` before it
    // flows into a marker filename and user-facing text. This rejects a
    // malformed / empty / equal / backwards `latest` (a backend inconsistency,
    // or a corrupted / hand-edited cache) that would otherwise (a) defeat the
    // per-version marker — `fs::write` of a filesystem-hostile or over-long
    // filename fails silently, so the nudge would re-fire every turn — or
    // (b) inject multi-line content into the user-facing systemMessage. NOTE:
    // this compares the CLI's own current↔latest (the correct baseline), NOT the
    // plugin-version comparison the CE-1 review removed.
    if !is_newer(&cache.latest, &cache.current) {
        return false;
    }
    if marker_exists {
        return false; // already nudged for this version
    }
    true
}

/// Agent-facing nudge text. Instructs the agent to fire an AskUserQuestion that
/// routes to the `update` skill (CLI binary upgrade), with a permanent opt-out.
fn nudge_text(current: &str, latest: &str) -> String {
    format!(
        "<axhub-cli-update>\n\
         [axhub hook | CLI 새 버전 알림]\n\
         axhub CLI 새 버전이 나왔어요: v{current} → v{latest}\n\
         에이전트 필수 동작: 이 턴의 다른 답변을 시작하기 전에 먼저 사용자에게 \
         AskUserQuestion 으로 업데이트할지 물어요. 질문 header 는 '업데이트' 로, \
         선택지는 '업데이트할래요', '지금은 그대로', '그만 볼래요 (다시 안 봄)' 를 \
         포함해요. '업데이트할래요' 는 update 스킬로 라우팅해요. '그만 볼래요 \
         (다시 안 봄)' 를 선택하면 `axhub-helpers cli-drift-optout` 를 실행해 영구로 \
         꺼요. AskUserQuestion 도구가 없으면 일반 채팅으로 같은 세 선택지를 묻고 \
         멈춰요.\n\
         Skip: AXHUB_DISABLE_HOOK=cli-drift\n\
         </axhub-cli-update>"
    )
}

/// User-facing fallback for the same drift nudge (UserPromptSubmit `systemMessage`).
fn nudge_system_message(current: &str, latest: &str) -> String {
    format!(
        "axhub CLI 새 버전이 나왔어요: v{current} → v{latest}\n\
         업데이트할까요? `업데이트할래요`, `지금은 그대로`, `그만 볼래요` 중 하나로 답해 주세요."
    )
}

/// Paired UserPromptSubmit outputs for a CLI drift event.
pub struct CliDriftNudge {
    pub additional_context: String,
    pub system_message: String,
}

/// Background fetch entry point (`axhub-helpers cli-latest-fetch-bg`).
/// Best-effort + TTL-gated; always returns 0 (fail-open hook contract).
pub fn cmd_cli_latest_fetch_bg() -> i32 {
    cmd_cli_latest_fetch_bg_with(fetch_cli_update_check)
}

fn cmd_cli_latest_fetch_bg_with(fetch: impl Fn() -> Option<CliLatestCache>) -> i32 {
    if is_hook_disabled("cli-drift") {
        return 0;
    }
    // Skip the subprocess entirely while the cache is still fresh.
    if let Some(cache) = read_cache() {
        if now_unix().saturating_sub(cache.fetched_at) < CACHE_TTL_SECS {
            return 0;
        }
    }
    if let Some(parsed) = fetch() {
        let _ = write_cache(&parsed);
    }
    0
}

/// Permanent opt-out (`axhub-helpers cli-drift-optout`). Writes the marker the
/// drift check honors. Always returns 0 (fail-open).
pub fn cmd_cli_drift_optout() -> i32 {
    if let Some(path) = cli_drift_optout_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, b"");
    }
    0
}

/// UserPromptSubmit nudge text, or `None` when no nudge should fire. Records the
/// per-version marker as a side effect on a `Some` result. Called from
/// `cmd_prompt_route` (after the plugin nudge, gated on plugin not firing and
/// the prompt not already being an update-check intent).
pub fn cli_drift_context() -> Option<String> {
    cli_drift_nudge().map(|n| n.additional_context)
}

/// UserPromptSubmit nudge payloads, or `None`. Records the per-version marker as
/// a side effect so the nudge fires at most once per latest version.
pub fn cli_drift_nudge() -> Option<CliDriftNudge> {
    if is_hook_disabled("cli-drift") {
        return None;
    }
    let cache = read_cache();
    let marker_path = cache
        .as_ref()
        .and_then(|c| cli_drift_nudge_marker_path(&c.latest));
    let marker_exists = marker_path.as_ref().map(|p| p.exists()).unwrap_or(false);

    if !cli_should_nudge(
        cache.as_ref(),
        now_unix(),
        marker_exists,
        optout_present(),
        is_non_interactive(),
    ) {
        return None;
    }

    // Record the per-version marker before returning so re-entry is a no-op.
    if let Some(path) = marker_path {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, b"");
    }
    cache.map(|c| CliDriftNudge {
        additional_context: nudge_text(&c.current, &c.latest),
        system_message: nudge_system_message(&c.current, &c.latest),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn cache(
        current: &str,
        latest: &str,
        has_update: bool,
        disabled: bool,
        at: u64,
    ) -> CliLatestCache {
        CliLatestCache {
            current: current.to_string(),
            latest: latest.to_string(),
            has_update,
            disabled,
            fetched_at: at,
        }
    }

    struct EnvGuard {
        vars: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvGuard {
        fn clear() -> Self {
            let keys = [
                "AXHUB_DISABLE_HOOKS",
                "AXHUB_DISABLE_HOOK",
                "DISABLE_AXHUB",
                "CI",
                "CLAUDE_NON_INTERACTIVE",
                "XDG_CACHE_HOME",
                "XDG_STATE_HOME",
            ];
            let vars = keys
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect();
            for key in keys {
                std::env::remove_var(key);
            }
            Self { vars }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.vars.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    // ── CE-1: version-source — current is the CLI's, not the plugin's ──

    #[test]
    fn parse_uses_cli_current_not_plugin_version_and_strips_v() {
        let parsed = parse_cli_update_check(
            r#"{"current":"v0.18.1","latest":"v0.18.2","has_update":true,"is_downgrade":false,"disabled":false}"#,
        )
        .unwrap();
        assert_eq!(parsed.current, "0.18.1"); // CLI version, not env!(CARGO_PKG_VERSION)
        assert_eq!(parsed.latest, "0.18.2");
        assert!(parsed.has_update);
        assert!(!parsed.disabled);
        // The plugin's CARGO_PKG_VERSION must NOT be what drives this channel.
        assert_ne!(parsed.current, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn nudge_requires_backend_has_update_and_a_strictly_newer_latest() {
        // Backend `has_update=false` is authoritative for the NO case — never
        // nudge, even if latest > current numerically.
        let no = cache("0.18.1", "0.18.9", false, false, 1000);
        assert!(!cli_should_nudge(Some(&no), 1000, false, false, false));
        // Backend `has_update=true` but `latest` is NOT strictly newer than the
        // CLI's own `current` (backwards / equal) → suppressed as a defense-in-
        // depth sanity gate (CLI current↔latest, not the plugin-version CE-1 trap).
        let backwards = cache("0.18.5", "0.18.2", true, false, 1000);
        assert!(!cli_should_nudge(
            Some(&backwards),
            1000,
            false,
            false,
            false
        ));
        let equal = cache("0.18.2", "0.18.2", true, false, 1000);
        assert!(!cli_should_nudge(Some(&equal), 1000, false, false, false));
        // has_update=true AND strictly newer → fire.
        let real = cache("0.18.1", "0.18.2", true, false, 1000);
        assert!(cli_should_nudge(Some(&real), 1000, false, false, false));
    }

    #[test]
    fn nudge_suppressed_for_malformed_or_hostile_latest() {
        // These would otherwise (a) defeat the per-version marker — fs::write of a
        // hostile/over-long filename fails silently → re-fire every turn — or
        // (b) inject multi-line content into the user-facing systemMessage.
        // The is_newer gate rejects every non-semver `latest` before it is used.
        let bad = |latest: &str| cache("0.18.1", latest, true, false, 1000);
        assert!(!cli_should_nudge(Some(&bad("")), 1000, false, false, false)); // empty → "v → v"
        assert!(!cli_should_nudge(
            Some(&bad("0.18.2\ninjected")),
            1000,
            false,
            false,
            false
        )); // newline injection
        assert!(!cli_should_nudge(
            Some(&bad("0.18.2\0evil")),
            1000,
            false,
            false,
            false
        )); // NUL → marker write fails
        let overlong = "9.".repeat(300); // > NAME_MAX → ENAMETOOLONG on marker write
        assert!(!cli_should_nudge(
            Some(&bad(&overlong)),
            1000,
            false,
            false,
            false
        ));
        assert!(!cli_should_nudge(
            Some(&bad("garbage")),
            1000,
            false,
            false,
            false
        ));
    }

    // ── parse fail-open ──

    #[test]
    fn parse_fails_open_on_malformed_empty_and_missing_current() {
        assert!(parse_cli_update_check("").is_none());
        assert!(parse_cli_update_check("not json").is_none());
        assert!(parse_cli_update_check(r#"{"latest":"v0.18.2"}"#).is_none()); // no current
        assert!(parse_cli_update_check(r#"{"current":""}"#).is_none()); // empty current
    }

    #[test]
    fn parse_disabled_returns_disabled_cache() {
        let parsed = parse_cli_update_check(
            r#"{"current":"v0.18.1","latest":"","has_update":false,"disabled":true}"#,
        )
        .unwrap();
        assert!(parsed.disabled);
        assert!(!parsed.has_update);
    }

    // ── cli_should_nudge gates ──

    #[test]
    fn should_nudge_fires_on_fresh_has_update_unmarked() {
        let c = cache("0.18.1", "0.18.2", true, false, 1000);
        assert!(cli_should_nudge(Some(&c), 1000, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_no_cache() {
        assert!(!cli_should_nudge(None, 1000, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_disabled() {
        let c = cache("0.18.1", "0.18.2", true, true, 1000);
        assert!(!cli_should_nudge(Some(&c), 1000, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_no_update() {
        let c = cache("0.18.1", "0.18.1", false, false, 1000);
        assert!(!cli_should_nudge(Some(&c), 1000, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_stale() {
        let c = cache("0.18.1", "0.18.2", true, false, 1000);
        let now = 1000 + CACHE_TTL_SECS; // exactly TTL → stale
        assert!(!cli_should_nudge(Some(&c), now, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_already_marked() {
        let c = cache("0.18.1", "0.18.2", true, false, 1000);
        assert!(!cli_should_nudge(Some(&c), 1000, true, false, false));
    }

    #[test]
    fn should_nudge_false_on_optout() {
        let c = cache("0.18.1", "0.18.2", true, false, 1000);
        assert!(!cli_should_nudge(Some(&c), 1000, false, true, false));
    }

    #[test]
    fn should_nudge_false_when_non_interactive() {
        let c = cache("0.18.1", "0.18.2", true, false, 1000);
        assert!(!cli_should_nudge(Some(&c), 1000, false, false, true));
    }

    // ── nudge text ──

    #[test]
    fn nudge_text_routes_to_update_skill_with_optout() {
        let t = nudge_text("0.18.1", "0.18.2");
        assert!(t.contains("v0.18.1"));
        assert!(t.contains("v0.18.2"));
        assert!(t.contains("CLI 새 버전 알림"));
        assert!(t.contains("update 스킬"));
        assert!(t.contains("cli-drift-optout"));
        assert!(t.contains("AXHUB_DISABLE_HOOK=cli-drift"));
        assert!(t.contains("그만 볼래요"));
    }

    #[test]
    fn nudge_system_message_is_user_facing_fallback() {
        let t = nudge_system_message("0.18.1", "0.18.2");
        assert!(t.contains("axhub CLI 새 버전이 나왔어요: v0.18.1 → v0.18.2"));
        assert!(t.contains("업데이트할까요?"));
        assert!(!t.contains("AskUserQuestion"));
        assert!(!t.contains("axhub-helpers"));
    }

    #[test]
    fn cache_roundtrip_serde() {
        let c = cache("0.18.1", "0.18.2", true, false, 12345);
        let json = serde_json::to_string(&c).unwrap();
        let back: CliLatestCache = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    // ── fetch-bg command (kill-switch / TTL / write) via injected fetcher ──

    #[test]
    fn fetch_command_disabled_by_kill_switch() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        std::env::set_var("AXHUB_DISABLE_HOOK", "cli-drift");

        let called = std::cell::Cell::new(false);
        assert_eq!(
            cmd_cli_latest_fetch_bg_with(|| {
                called.set(true);
                Some(cache("0.18.1", "0.18.2", true, false, now_unix()))
            }),
            0
        );
        assert!(!called.get(), "kill switch must skip the fetch entirely");
        assert!(read_cache().is_none());
    }

    #[test]
    fn fetch_command_skips_while_cache_fresh() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("0.18.1", "0.18.1", false, false, now_unix())).unwrap();

        let called = std::cell::Cell::new(false);
        assert_eq!(
            cmd_cli_latest_fetch_bg_with(|| {
                called.set(true);
                Some(cache("0.18.1", "0.18.2", true, false, now_unix()))
            }),
            0
        );
        assert!(!called.get(), "fresh cache must skip the fetch");
        assert!(!read_cache().unwrap().has_update); // untouched
    }

    #[test]
    fn fetch_command_refreshes_stale_cache() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache(
            "0.18.1",
            "0.18.1",
            false,
            false,
            now_unix() - CACHE_TTL_SECS - 1,
        ))
        .unwrap();

        assert_eq!(
            cmd_cli_latest_fetch_bg_with(|| Some(cache(
                "0.18.1",
                "0.18.2",
                true,
                false,
                now_unix()
            ))),
            0
        );
        let refreshed = read_cache().unwrap();
        assert_eq!(refreshed.latest, "0.18.2");
        assert!(refreshed.has_update);
    }

    #[test]
    fn fetch_command_returns_zero_when_fetch_yields_none() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());

        assert_eq!(cmd_cli_latest_fetch_bg_with(|| None), 0);
        assert!(read_cache().is_none());
    }

    #[test]
    fn write_cache_is_atomic_and_roundtrips() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());

        write_cache(&cache("0.18.1", "0.18.2", true, false, 12345)).unwrap();

        let path = cli_latest_cache_path().unwrap();
        assert_eq!(
            read_cache(),
            Some(cache("0.18.1", "0.18.2", true, false, 12345))
        );
        assert!(path.exists());
        assert!(
            !path.with_extension("json.tmp").exists(),
            "atomic temp file should not remain after rename"
        );
    }

    // ── nudge integration (marker dedup + optout) ──

    #[test]
    fn cli_drift_nudge_fires_once_then_dedups_via_marker() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("0.18.1", "0.18.2", true, false, now_unix())).unwrap();

        let first = cli_drift_nudge();
        assert!(first.is_some(), "first turn fires");
        assert!(first.unwrap().additional_context.contains("v0.18.2"));
        // marker now written → second turn is a no-op (natural yield to plugin/next).
        assert!(cli_drift_nudge().is_none(), "per-version marker dedups");
    }

    #[test]
    fn cli_drift_nudge_none_and_no_refire_for_hostile_cache_latest() {
        // A cache (corrupt / hand-edited / buggy backend) with has_update=true but
        // an over-long `latest` must NOT nudge, must NOT panic, and must NOT
        // re-fire: the is_newer gate suppresses it before the marker-write path.
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        let overlong = "9.".repeat(300);
        write_cache(&cache("0.18.1", &overlong, true, false, now_unix())).unwrap();

        assert!(cli_drift_nudge().is_none(), "hostile latest must not nudge");
        // No marker leaked, and a second turn is still None (no re-fire loop).
        assert!(
            cli_drift_nudge().is_none(),
            "still suppressed next turn (no re-fire)"
        );
        assert!(
            cli_drift_nudge_marker_path(&overlong)
                .map(|p| !p.exists())
                .unwrap_or(true),
            "no junk marker written for hostile latest"
        );
    }

    #[test]
    fn cli_drift_nudge_none_on_truncated_cache_file() {
        // Hostile/truncated cache content → read_cache swallows the parse error →
        // None, exit-0 contract, no panic.
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        let path = cli_latest_cache_path().unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, br#"{"current":"0.18.1","latest":"0.1"#).unwrap(); // truncated

        assert!(read_cache().is_none());
        assert!(cli_drift_nudge().is_none());
    }

    #[test]
    fn cli_drift_nudge_none_when_disabled_cache() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("0.18.1", "0.18.2", true, true, now_unix())).unwrap();

        assert!(cli_drift_nudge().is_none());
    }

    #[test]
    fn cli_drift_nudge_none_after_optout() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("0.18.1", "0.18.2", true, false, now_unix())).unwrap();

        assert_eq!(cmd_cli_drift_optout(), 0);
        assert!(cli_drift_optout_path().unwrap().exists());
        assert!(
            cli_drift_nudge().is_none(),
            "opt-out suppresses every version"
        );
    }

    #[test]
    fn cli_drift_nudge_disabled_by_kill_switch() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("0.18.1", "0.18.2", true, false, now_unix())).unwrap();
        std::env::set_var("AXHUB_DISABLE_HOOK", "cli-drift");

        assert!(cli_drift_nudge().is_none());
    }

    #[test]
    fn optout_command_writes_marker() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());

        assert_eq!(cmd_cli_drift_optout(), 0);
        assert!(cli_drift_optout_path().unwrap().exists());
    }
}

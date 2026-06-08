//! Proactive plugin version-drift nudge (plan: docs/plans/plugin-update-proactive-nudge.md).
//!
//! Data flow — every path is fail-open (any error → no nudge, exit 0):
//!
//! ```text
//!   plugin-latest-fetch-bg  (spawned detached from session-start.sh)
//!     ureq GET releases/latest ──▶ parse tag_name ──▶ atomic cache write (TTL 24h)
//!        │                            │                   │
//!        ▼                            ▼                   ▼
//!     [net fail] → skip          [malformed] → skip   [io fail] → skip
//!
//!   prompt-route  (UserPromptSubmit — the reliable steering surface; D4)
//!     read cache ──▶ semver vs CARGO_PKG_VERSION ──▶ Some(nudge) + per-version marker
//!        │              │                              │
//!        ▼              ▼                              ▼
//!     [absent/stale] [<= current]                 [marker/optout/non-interactive] → None
//! ```
//!
//! D4: the nudge rides **UserPromptSubmit** `additionalContext`, NOT SessionStart.
//! SessionStart `additionalContext` is advisory and arrives before any user turn,
//! so it cannot reliably drive an AskUserQuestion. UserPromptSubmit fires with a
//! real turn (the surface `prompt-route` already uses to steer the agent).

use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::hook_safety::is_hook_disabled;
use crate::runtime_paths::{
    plugin_drift_nudge_marker_path, plugin_drift_optout_path, plugin_latest_cache_path,
};

const RELEASES_LATEST_URL: &str =
    "https://api.github.com/repos/jocoding-ax-partners/axhub/releases/latest";
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;
const FETCH_TIMEOUT_SECS: u64 = 5;

/// Cached result of the most recent successful latest-release fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestCache {
    /// Normalized semver, no leading `v` (e.g. `"0.9.40"`).
    pub latest: String,
    /// Unix seconds at which the fetch succeeded.
    pub fetched_at: u64,
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Current plugin version (compile-time constant; plugin + helper co-version,
/// see telemetry::PLUGIN_VERSION).
fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Strip a leading `v` and surrounding whitespace from a release tag.
pub(crate) fn normalize_tag(tag: &str) -> String {
    tag.trim().trim_start_matches('v').to_string()
}

/// `true` when `latest` is a strictly newer semver than `current`. Unparseable
/// input yields `false` (fail-open: never nudge on garbage).
pub(crate) fn is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(l), Ok(c)) => l > c,
        _ => false,
    }
}

/// Pure drift decision — no IO, so every branch is unit-testable. Returns `true`
/// when the nudge should fire this turn.
fn should_nudge(
    cache: Option<&LatestCache>,
    current: &str,
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
    if now.saturating_sub(cache.fetched_at) >= CACHE_TTL_SECS {
        return false; // stale cache
    }
    if !is_newer(&cache.latest, current) {
        return false; // already current (or a preview/downgrade)
    }
    if marker_exists {
        return false; // already nudged for this version
    }
    true
}

fn optout_present() -> bool {
    plugin_drift_optout_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

pub(crate) fn is_non_interactive() -> bool {
    std::env::var_os("CI").is_some() || std::env::var_os("CLAUDE_NON_INTERACTIVE").is_some()
}

fn read_cache() -> Option<LatestCache> {
    let path = plugin_latest_cache_path()?;
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Atomic cache write: serialize to a sibling temp file then rename (atomic on
/// the same filesystem), so a concurrent reader never sees a torn file.
fn write_cache(cache: &LatestCache) -> std::io::Result<()> {
    let path = plugin_latest_cache_path()
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

fn fetch_latest_tag_from_url(url: &str) -> Option<String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(FETCH_TIMEOUT_SECS)))
        .build();
    let agent: ureq::Agent = config.into();
    let mut resp = agent
        .get(url)
        .header("User-Agent", "axhub-helpers")
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?;
    let body = resp.body_mut().read_to_string().ok()?;
    let value: serde_json::Value = serde_json::from_str(&body).ok()?;
    let tag = value.get("tag_name").and_then(serde_json::Value::as_str)?;
    let normalized = normalize_tag(tag);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Agent-facing nudge text. Instructs the agent to fire an AskUserQuestion that
/// routes to the `upgrade` skill, and to include the permanent opt-out option.
fn nudge_text(current: &str, latest: &str) -> String {
    format!(
        "<axhub-plugin-update>\n\
         [axhub hook | 플러그인 새 버전 알림]\n\
         플러그인 새 버전이 나왔어요: v{current} → v{latest}\n\
         에이전트 필수 동작: 이 턴의 다른 답변을 시작하기 전에 먼저 사용자에게 \
         AskUserQuestion 으로 업데이트할지 물어요. 질문 header 는 '업데이트' 로, \
         선택지는 '업데이트할래요', '지금은 그대로', '그만 볼래요 (다시 안 봄)' 를 \
         포함해요. '업데이트할래요' 는 upgrade 스킬로 라우팅해요. '그만 볼래요 \
         (다시 안 봄)' 를 선택하면 `axhub-helpers plugin-drift-optout` 를 실행해 영구로 \
         꺼요. AskUserQuestion 도구가 없으면 일반 채팅으로 같은 세 선택지를 묻고 \
         멈춰요.\n\
         Skip: AXHUB_DISABLE_HOOK=plugin-drift\n\
         </axhub-plugin-update>"
    )
}

/// User-facing fallback for the same drift nudge. This rides on the
/// UserPromptSubmit `systemMessage` channel so live users still see the update
/// notice even when the agent ignores `additionalContext` or cannot render an
/// AskUserQuestion card.
fn nudge_system_message(current: &str, latest: &str) -> String {
    format!(
        "플러그인 새 버전이 나왔어요: v{current} → v{latest}\n\
         업데이트할까요? `업데이트할래요`, `지금은 그대로`, `그만 볼래요` 중 하나로 답해 주세요."
    )
}

/// Paired UserPromptSubmit outputs for a plugin drift event.
pub struct PluginDriftNudge {
    pub additional_context: String,
    pub system_message: String,
}

/// Background fetch entry point (`axhub-helpers plugin-latest-fetch-bg`).
/// Best-effort + TTL-gated; always returns 0 (fail-open hook contract).
pub fn cmd_plugin_latest_fetch_bg() -> i32 {
    cmd_plugin_latest_fetch_bg_with_url(RELEASES_LATEST_URL)
}

fn cmd_plugin_latest_fetch_bg_with_url(url: &str) -> i32 {
    if is_hook_disabled("plugin-drift") {
        return 0;
    }
    // Skip the network call entirely while the cache is still fresh.
    if let Some(cache) = read_cache() {
        if now_unix().saturating_sub(cache.fetched_at) < CACHE_TTL_SECS {
            return 0;
        }
    }
    if let Some(latest) = fetch_latest_tag_from_url(url) {
        let _ = write_cache(&LatestCache {
            latest,
            fetched_at: now_unix(),
        });
    }
    0
}

/// Explicit plugin update check (`axhub-helpers plugin-update-check --json`).
/// Called by the `upgrade` skill on an explicit user request. Unlike the passive
/// plugin-drift nudge, this does a **fresh** GitHub releases fetch — the bundled
/// `.claude-plugin/marketplace.json` is stale-by-design (it ships *with* the
/// plugin, so it always reports the installed version as the latest). Mirrors the
/// CLI's `axhub update check --json` contract. Always exits 0; emits one JSON line:
///   `{"current":"0.9.37","latest":"0.9.38","has_update":true,"checked":true}`
/// On a network/parse failure `checked` is `false` and `latest` is `null`, so the
/// skill can say "couldn't check" instead of a false "up to date".
pub fn cmd_plugin_update_check() -> i32 {
    cmd_plugin_update_check_with_url(RELEASES_LATEST_URL)
}

fn cmd_plugin_update_check_with_url(url: &str) -> i32 {
    println!("{}", plugin_update_check_payload(url));
    0
}

fn plugin_update_check_payload(url: &str) -> serde_json::Value {
    let current = current_version();
    match fetch_latest_tag_from_url(url) {
        Some(latest) => {
            let has_update = is_newer(&latest, current);
            // Warm the nudge cache as a side benefit so the proactive nudge and
            // this explicit check agree on the latest version.
            let _ = write_cache(&LatestCache {
                latest: latest.clone(),
                fetched_at: now_unix(),
            });
            serde_json::json!({
                "current": current,
                "latest": latest,
                "has_update": has_update,
                "checked": true,
            })
        }
        None => serde_json::json!({
            "current": current,
            "latest": serde_json::Value::Null,
            "has_update": false,
            "checked": false,
        }),
    }
}

/// Permanent opt-out (`axhub-helpers plugin-drift-optout`). Writes the marker the
/// drift check honors. Always returns 0 (fail-open).
pub fn cmd_plugin_drift_optout() -> i32 {
    if let Some(path) = plugin_drift_optout_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, b"");
    }
    0
}

/// UserPromptSubmit nudge text, or `None` when no nudge should fire. On a `Some`
/// result the per-version marker is recorded as a side effect, so the nudge
/// fires at most once per latest version. Called from `cmd_prompt_route`.
pub fn plugin_drift_context() -> Option<String> {
    plugin_drift_nudge().map(|n| n.additional_context)
}

/// UserPromptSubmit nudge payloads, or `None` when no nudge should fire. On a
/// `Some` result the per-version marker is recorded as a side effect, so the
/// nudge fires at most once per latest version.
pub fn plugin_drift_nudge() -> Option<PluginDriftNudge> {
    if is_hook_disabled("plugin-drift") {
        return None;
    }
    let cache = read_cache();
    let current = current_version();
    let marker_path = cache
        .as_ref()
        .and_then(|c| plugin_drift_nudge_marker_path(&c.latest));
    let marker_exists = marker_path.as_ref().map(|p| p.exists()).unwrap_or(false);

    if !should_nudge(
        cache.as_ref(),
        current,
        now_unix(),
        marker_exists,
        optout_present(),
        is_non_interactive(),
    ) {
        return None;
    }

    // Record the per-version marker before returning so re-entry (this turn's
    // later prompts, or the next session) is a no-op.
    if let Some(path) = marker_path {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, b"");
    }
    // `cache` is Some here (should_nudge returned true).
    cache.map(|c| PluginDriftNudge {
        additional_context: nudge_text(current, &c.latest),
        system_message: nudge_system_message(current, &c.latest),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    fn cache(latest: &str, fetched_at: u64) -> LatestCache {
        LatestCache {
            latest: latest.to_string(),
            fetched_at,
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

    fn serve_once(status: &str, body: &str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_string();
        let body = body.to_string();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            drain_request(&mut stream);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{addr}/latest"), handle)
    }

    fn serve_connection_probe(status: &str, body: &str) -> (String, thread::JoinHandle<bool>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_string();
        let body = body.to_string();
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_millis(150);
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        drain_request(&mut stream);
                        let response = format!(
                            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                            body.len()
                        );
                        stream.write_all(response.as_bytes()).unwrap();
                        return true;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => return false,
                }
            }
            false
        });
        (format!("http://{addr}/latest"), handle)
    }

    fn drain_request(stream: &mut TcpStream) {
        let mut buf = [0_u8; 1024];
        let _ = stream.read(&mut buf);
    }

    fn closed_local_url() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        format!("http://{addr}/latest")
    }

    #[test]
    fn normalize_tag_strips_v_and_space() {
        assert_eq!(normalize_tag("v0.9.40"), "0.9.40");
        assert_eq!(normalize_tag("  0.9.40 "), "0.9.40");
        assert_eq!(normalize_tag("v1.2.3"), "1.2.3");
    }

    #[test]
    fn is_newer_compares_semver_numerically() {
        assert!(is_newer("0.9.40", "0.9.34"));
        assert!(is_newer("0.10.0", "0.9.99")); // not string compare
        assert!(!is_newer("0.9.34", "0.9.34")); // equal
        assert!(!is_newer("0.9.30", "0.9.34")); // older (preview/downgrade)
        assert!(!is_newer("garbage", "0.9.34")); // unparseable → no nudge
        assert!(!is_newer("0.9.40", "garbage"));
    }

    #[test]
    fn should_nudge_fires_on_fresh_newer_unmarked() {
        let c = cache("0.9.40", 1000);
        assert!(should_nudge(Some(&c), "0.9.34", 1000, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_no_cache() {
        assert!(!should_nudge(None, "0.9.34", 1000, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_equal_or_older() {
        let eq = cache("0.9.34", 1000);
        assert!(!should_nudge(
            Some(&eq),
            "0.9.34",
            1000,
            false,
            false,
            false
        ));
        let older = cache("0.9.30", 1000);
        assert!(!should_nudge(
            Some(&older),
            "0.9.34",
            1000,
            false,
            false,
            false
        ));
    }

    #[test]
    fn should_nudge_false_when_stale() {
        let c = cache("0.9.40", 1000);
        let now = 1000 + CACHE_TTL_SECS; // exactly TTL → stale
        assert!(!should_nudge(Some(&c), "0.9.34", now, false, false, false));
    }

    #[test]
    fn should_nudge_false_when_already_marked() {
        let c = cache("0.9.40", 1000);
        assert!(!should_nudge(Some(&c), "0.9.34", 1000, true, false, false));
    }

    #[test]
    fn should_nudge_false_on_optout() {
        let c = cache("0.9.40", 1000);
        assert!(!should_nudge(Some(&c), "0.9.34", 1000, false, true, false));
    }

    #[test]
    fn should_nudge_false_when_non_interactive() {
        let c = cache("0.9.40", 1000);
        assert!(!should_nudge(Some(&c), "0.9.34", 1000, false, false, true));
    }

    #[test]
    fn nudge_text_contains_versions_and_optout() {
        let t = nudge_text("0.9.34", "0.9.40");
        assert!(t.contains("v0.9.34"));
        assert!(t.contains("v0.9.40"));
        assert!(t.contains("에이전트 필수 동작"));
        assert!(t.contains("이 턴의 다른 답변을 시작하기 전에 먼저"));
        assert!(t.contains("업데이트할래요"));
        assert!(t.contains("지금은 그대로"));
        assert!(t.contains("그만 볼래요"));
        assert!(t.contains("plugin-drift-optout"));
        assert!(t.contains("AXHUB_DISABLE_HOOK=plugin-drift"));
    }

    #[test]
    fn nudge_system_message_is_user_facing_fallback() {
        let t = nudge_system_message("0.9.34", "0.9.40");
        assert!(t.contains("플러그인 새 버전이 나왔어요: v0.9.34 → v0.9.40"));
        assert!(t.contains("업데이트할까요?"));
        assert!(t.contains("업데이트할래요"));
        assert!(t.contains("지금은 그대로"));
        assert!(t.contains("그만 볼래요"));
        assert!(!t.contains("AskUserQuestion"));
        assert!(!t.contains("axhub-helpers"));
    }

    #[test]
    fn cache_roundtrip_serde() {
        let c = cache("0.9.40", 12345);
        let json = serde_json::to_string(&c).unwrap();
        let back: LatestCache = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn fetch_latest_tag_from_url_reads_and_normalizes_release_tag() {
        let (url, handle) = serve_once("200 OK", r#"{"tag_name":"v99.0.0"}"#);

        assert_eq!(fetch_latest_tag_from_url(&url), Some("99.0.0".to_string()));

        handle.join().unwrap();
    }

    #[test]
    fn fetch_latest_tag_from_url_fails_open_for_rate_limit_malformed_and_network() {
        let (rate_url, rate_handle) = serve_once("429 Too Many Requests", r#"{"message":"rate"}"#);
        assert_eq!(fetch_latest_tag_from_url(&rate_url), None);
        rate_handle.join().unwrap();

        let (malformed_url, malformed_handle) = serve_once("200 OK", r#"{"name":"missing tag"}"#);
        assert_eq!(fetch_latest_tag_from_url(&malformed_url), None);
        malformed_handle.join().unwrap();

        assert_eq!(fetch_latest_tag_from_url(&closed_local_url()), None);
    }

    #[test]
    fn write_cache_is_atomic_and_roundtrips() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());

        write_cache(&cache("99.0.0", 12345)).unwrap();

        let path = plugin_latest_cache_path().unwrap();
        assert_eq!(read_cache(), Some(cache("99.0.0", 12345)));
        assert!(path.exists());
        assert!(
            !path.with_extension("json.tmp").exists(),
            "atomic temp file should not remain after successful rename"
        );
    }

    #[test]
    fn fetch_command_skips_network_while_cache_is_fresh() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("88.0.0", now_unix())).unwrap();
        let (url, handle) = serve_connection_probe("200 OK", r#"{"tag_name":"v99.0.0"}"#);

        assert_eq!(cmd_plugin_latest_fetch_bg_with_url(&url), 0);

        assert_eq!(read_cache().unwrap().latest, "88.0.0");
        assert!(
            !handle.join().unwrap(),
            "fresh cache should skip HTTP fetch"
        );
    }

    #[test]
    fn fetch_command_refreshes_stale_cache() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        write_cache(&cache("88.0.0", now_unix() - CACHE_TTL_SECS - 1)).unwrap();
        let (url, handle) = serve_once("200 OK", r#"{"tag_name":"v99.0.0"}"#);

        assert_eq!(cmd_plugin_latest_fetch_bg_with_url(&url), 0);

        assert_eq!(read_cache().unwrap().latest, "99.0.0");
        handle.join().unwrap();
    }

    #[test]
    fn fetch_command_returns_zero_when_cache_write_fails() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_home_file = tempfile::NamedTempFile::new().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_home_file.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        let (url, handle) = serve_once("200 OK", r#"{"tag_name":"v99.0.0"}"#);

        assert_eq!(cmd_plugin_latest_fetch_bg_with_url(&url), 0);
        assert!(read_cache().is_none());

        handle.join().unwrap();
    }

    #[test]
    fn optout_command_writes_marker() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());

        assert_eq!(cmd_plugin_drift_optout(), 0);

        assert!(plugin_drift_optout_path().unwrap().exists());
    }

    // ── plugin-update-check (explicit fresh check for the upgrade skill) ──

    #[test]
    fn update_check_reports_has_update_on_newer_remote() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        let (url, handle) = serve_once("200 OK", r#"{"tag_name":"v99.0.0"}"#);

        let p = plugin_update_check_payload(&url);
        assert_eq!(p["current"], current_version());
        assert_eq!(p["latest"], "99.0.0");
        assert_eq!(p["has_update"], true);
        assert_eq!(p["checked"], true);
        // side benefit: the explicit check warms the nudge cache.
        assert_eq!(read_cache().unwrap().latest, "99.0.0");

        handle.join().unwrap();
    }

    #[test]
    fn update_check_reports_up_to_date_when_remote_equals_current() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        let body = format!(r#"{{"tag_name":"v{}"}}"#, current_version());
        let (url, handle) = serve_once("200 OK", &body);

        let p = plugin_update_check_payload(&url);
        assert_eq!(p["has_update"], false);
        assert_eq!(p["checked"], true);

        handle.join().unwrap();
    }

    #[test]
    fn update_check_reports_checked_false_on_network_failure() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());

        let p = plugin_update_check_payload(&closed_local_url());
        // No false "up to date" — the skill distinguishes checked:false.
        assert_eq!(p["checked"], false);
        assert_eq!(p["has_update"], false);
        assert!(p["latest"].is_null());
        assert!(
            read_cache().is_none(),
            "failed check must not warm the cache"
        );
    }

    #[test]
    fn update_check_command_exits_zero() {
        let _guard = crate::PROCESS_ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::clear();
        let cache_dir = tempfile::tempdir().unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", cache_dir.path());
        std::env::set_var("XDG_STATE_HOME", state_dir.path());
        assert_eq!(cmd_plugin_update_check_with_url(&closed_local_url()), 0);
    }
}

//! Pure, read-only tenant resolver for the tenant-picker.
//!
//! Resolution precedence:
//!   1. project-local cache (`.axhub/state/tenant.json`) if fresh
//!   2. `axhub tenants list --json` (auto-pick on a single tenant, else hand
//!      the candidates back so the bash layer can prompt)
//!   3. preflight / `axhub auth status --json` active team id
//!
//! This helper is intentionally read-only: it NEVER writes or deletes the
//! cache (the bash layer owns persistence). The cache `ts` is a typed `i64`,
//! so a non-numeric timestamp simply fails deserialization and the whole
//! cache is treated as missing/stale — there is no string-to-number coercion
//! path for the old bash arithmetic-injection class to live in.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::axhub_cli::{axhub_bin_from_env, run_axhub_with_timeout, DEFAULT_AXHUB_TIMEOUT};

pub const TENANT_STATE_RELATIVE_PATH: &str = ".axhub/state/tenant.json";
const DEFAULT_TENANT_CACHE_TTL_SECS: i64 = 28_800; // 8h
const TENANT_CACHE_TTL_ENV: &str = "AXHUB_TENANT_CACHE_TTL_SECS";

/// Tolerant view of the on-disk cache. Every field defaults so a partial blob
/// still deserializes; a missing `schema_version` is valid (not corrupt). `ts`
/// is a typed `i64` — a non-numeric JSON timestamp fails serde, which the
/// caller treats as a missing/stale cache rather than a panic.
#[derive(Debug, Clone, Deserialize)]
struct TenantCache {
    #[serde(default)]
    tenant: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    ts: i64,
    #[serde(default)]
    #[allow(dead_code)]
    schema_version: Option<String>,
}

/// Result of pure resolution, independent of the real filesystem/process.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolveOutput {
    tenant: String,
    source: String,
    needs_pick: bool,
    candidates: Vec<Value>,
}

impl ResolveOutput {
    fn empty() -> Self {
        Self {
            tenant: String::new(),
            source: String::new(),
            needs_pick: false,
            candidates: Vec::new(),
        }
    }
}

/// Pure resolver. Touches no real filesystem-cwd or real `axhub`; all effects
/// are injected so this is exhaustively unit-testable.
///
/// `list_tenants` returns `None` when the listing failed/timed out, and
/// `current_team_id` is only consulted when the listing yields nothing.
fn resolve_with(
    cache_json: Option<&str>,
    now_secs: i64,
    ttl_secs: i64,
    list_tenants: impl FnOnce() -> Option<Vec<Value>>,
    current_team_id: impl FnOnce() -> Option<String>,
) -> ResolveOutput {
    // Precedence 1: fresh cache.
    if let Some(out) = cache_json.and_then(|raw| cache_hit(raw, now_secs, ttl_secs)) {
        return out;
    }

    // Precedence 2: tenant listing.
    if let Some(tenants) = list_tenants() {
        match tenants.len() {
            0 => {}
            1 => {
                let tenant = tenant_identifier(&tenants[0]);
                return ResolveOutput {
                    tenant,
                    source: "auto".to_string(),
                    needs_pick: false,
                    candidates: Vec::new(),
                };
            }
            _ => {
                return ResolveOutput {
                    tenant: String::new(),
                    source: "list".to_string(),
                    needs_pick: true,
                    candidates: tenants,
                };
            }
        }
    }

    // Precedence 3: preflight active team id.
    match current_team_id() {
        Some(id) if !id.is_empty() => ResolveOutput {
            tenant: id,
            source: "preflight".to_string(),
            needs_pick: false,
            candidates: Vec::new(),
        },
        _ => ResolveOutput::empty(),
    }
}

/// Parse the raw cache and return a hit only when it is fresh: a non-empty
/// tenant and `now - ts` within `[0, ttl)`. Any parse failure (including a
/// non-numeric `ts`) yields `None` so the caller falls through to listing.
fn cache_hit(raw: &str, now_secs: i64, ttl_secs: i64) -> Option<ResolveOutput> {
    let cache: TenantCache = serde_json::from_str(raw).ok()?;
    if cache.tenant.is_empty() {
        return None;
    }
    let age = now_secs.checked_sub(cache.ts)?;
    if age < 0 || age >= ttl_secs {
        return None;
    }
    let source = if cache.source.is_empty() {
        "cache".to_string()
    } else {
        cache.source
    };
    Some(ResolveOutput {
        tenant: cache.tenant,
        source,
        needs_pick: false,
        candidates: Vec::new(),
    })
}

/// Pick the stable identifier from a tenant object: `id`, else `slug`, else "".
fn tenant_identifier(tenant: &Value) -> String {
    tenant
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| tenant.get("slug").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

/// Read TTL from the override env, falling back to the default on parse error
/// or when unset.
fn ttl_secs_from_env() -> i64 {
    std::env::var(TENANT_CACHE_TTL_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .unwrap_or(DEFAULT_TENANT_CACHE_TTL_SECS)
}

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Real `axhub tenants list --json` provider. Returns `None` on timeout,
/// non-zero exit, or unparseable / non-array stdout.
fn list_tenants_via_axhub() -> Option<Vec<Value>> {
    let out = run_axhub_with_timeout(
        &axhub_bin_from_env(),
        &["tenants", "list", "--json"],
        DEFAULT_AXHUB_TIMEOUT,
    );
    if out.timed_out || out.exit_code != 0 {
        return None;
    }
    match serde_json::from_str::<Value>(&out.stdout) {
        Ok(Value::Array(items)) => Some(items),
        _ => None,
    }
}

/// Real preflight provider: read the active team id from
/// `axhub auth status --json` (`.current_team_id`). Returns `None` on any
/// failure so resolution can fall through to an empty result.
fn current_team_id_via_axhub() -> Option<String> {
    let out = run_axhub_with_timeout(
        &axhub_bin_from_env(),
        &["auth", "status", "--json"],
        DEFAULT_AXHUB_TIMEOUT,
    );
    if out.timed_out || out.exit_code != 0 {
        return None;
    }
    let parsed: Value = serde_json::from_str(&out.stdout).ok()?;
    let id = parsed
        .get("current_team_id")
        .and_then(Value::as_str)?
        .to_string();
    (!id.is_empty()).then_some(id)
}

/// Inner worker that may use `?`; wrapped by the fail-open public entry point.
fn resolve_real() -> anyhow::Result<ResolveOutput> {
    let cache_path = std::env::current_dir()?.join(TENANT_STATE_RELATIVE_PATH);
    let cache_json = match std::fs::read_to_string(&cache_path) {
        Ok(raw) => Some(raw),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(err.into()),
    };
    Ok(resolve_with(
        cache_json.as_deref(),
        now_epoch_secs(),
        ttl_secs_from_env(),
        list_tenants_via_axhub,
        current_team_id_via_axhub,
    ))
}

fn output_json(out: &ResolveOutput) -> Value {
    json!({
        "tenant": out.tenant,
        "source": out.source,
        "needs_pick": out.needs_pick,
        "candidates": out.candidates,
    })
}

/// Resolve the tenant and print the result as JSON on stdout.
///
/// Accepts an optional `--json` flag for parity with sibling helpers (the
/// resolver always emits JSON). Fail-open: any internal error prints the empty
/// result and still returns `Ok(0)` so the picker skill is never crashed.
pub fn run_tenant_resolve(args: &[String]) -> anyhow::Result<i32> {
    for arg in args {
        if arg != "--json" {
            eprintln!("axhub-helpers tenant-resolve: unknown option \"{arg}\"");
        }
    }
    let out = resolve_real().unwrap_or_else(|_| ResolveOutput::empty());
    let value = output_json(&out);
    match serde_json::to_string(&value) {
        Ok(rendered) => println!("{rendered}"),
        Err(_) => {
            println!("{{\"tenant\":\"\",\"source\":\"\",\"needs_pick\":false,\"candidates\":[]}}")
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TTL: i64 = DEFAULT_TENANT_CACHE_TTL_SECS;

    fn no_list() -> Option<Vec<Value>> {
        None
    }

    fn no_team() -> Option<String> {
        None
    }

    #[test]
    fn fresh_cache_is_a_hit_and_preserves_source() {
        let now = 1_000_000;
        let cache = json!({
            "tenant": "team-acme",
            "source": "manual",
            "ts": now - 10,
            "schema_version": "tenant/v1"
        })
        .to_string();

        let out = resolve_with(Some(&cache), now, TTL, no_list, no_team);
        assert_eq!(out.tenant, "team-acme");
        assert_eq!(out.source, "manual");
        assert!(!out.needs_pick);
        assert!(out.candidates.is_empty());
    }

    #[test]
    fn cache_without_source_defaults_to_cache() {
        let now = 1_000_000;
        let cache = json!({ "tenant": "team-acme", "ts": now - 10 }).to_string();
        let out = resolve_with(Some(&cache), now, TTL, no_list, no_team);
        assert_eq!(out.tenant, "team-acme");
        assert_eq!(out.source, "cache");
    }

    #[test]
    fn ttl_stale_cache_falls_through_to_list() {
        let now = 1_000_000;
        let cache = json!({
            "tenant": "team-acme",
            "source": "manual",
            "ts": now - TTL
        })
        .to_string();

        let list = || Some(vec![json!({ "id": "team-from-list" })]);
        let out = resolve_with(Some(&cache), now, TTL, list, no_team);
        assert_eq!(out.tenant, "team-from-list");
        assert_eq!(out.source, "auto");
    }

    #[test]
    fn future_dated_cache_is_not_a_hit() {
        let now = 1_000_000;
        let cache = json!({ "tenant": "team-acme", "ts": now + 50 }).to_string();
        let list = || Some(vec![json!({ "id": "team-from-list" })]);
        let out = resolve_with(Some(&cache), now, TTL, list, no_team);
        assert_eq!(out.tenant, "team-from-list");
        assert_eq!(out.source, "auto");
    }

    #[test]
    fn legacy_cache_without_schema_version_is_still_valid() {
        let now = 1_000_000;
        // No schema_version, numeric ts — must be a valid hit, NOT corrupt.
        let cache = json!({
            "tenant": "team-legacy",
            "source": "manual",
            "ts": now - 5
        })
        .to_string();

        let out = resolve_with(Some(&cache), now, TTL, no_list, no_team);
        assert_eq!(out.tenant, "team-legacy");
        assert_eq!(out.source, "manual");
        assert!(!out.needs_pick);
    }

    #[test]
    fn malformed_cache_json_falls_through_without_panic() {
        let now = 1_000_000;
        let list = || Some(vec![json!({ "id": "team-from-list" })]);
        let out = resolve_with(Some("{not json"), now, TTL, list, no_team);
        assert_eq!(out.tenant, "team-from-list");
        assert_eq!(out.source, "auto");
    }

    #[test]
    fn non_numeric_ts_is_treated_as_stale_not_panic() {
        let now = 1_000_000;
        // A string ts fails serde (i64), so the whole cache is ignored.
        let cache = r#"{"tenant":"team-acme","source":"manual","ts":"not-a-number"}"#;
        let list = || Some(vec![json!({ "id": "team-from-list" })]);
        let out = resolve_with(Some(cache), now, TTL, list, no_team);
        assert_eq!(out.tenant, "team-from-list");
        assert_eq!(out.source, "auto");
    }

    #[test]
    fn empty_tenant_in_cache_falls_through() {
        let now = 1_000_000;
        let cache = json!({ "tenant": "", "source": "manual", "ts": now - 5 }).to_string();
        let out = resolve_with(Some(&cache), now, TTL, no_list, no_team);
        assert_eq!(out.tenant, "");
        assert_eq!(out.source, "");
        assert!(!out.needs_pick);
    }

    #[test]
    fn list_count_one_auto_picks_id() {
        let now = 1_000_000;
        let list = || Some(vec![json!({ "id": "solo-team", "slug": "solo" })]);
        let out = resolve_with(None, now, TTL, list, no_team);
        assert_eq!(out.tenant, "solo-team");
        assert_eq!(out.source, "auto");
        assert!(!out.needs_pick);
        assert!(out.candidates.is_empty());
    }

    #[test]
    fn list_count_one_falls_back_to_slug_when_no_id() {
        let now = 1_000_000;
        let list = || Some(vec![json!({ "slug": "slug-only" })]);
        let out = resolve_with(None, now, TTL, list, no_team);
        assert_eq!(out.tenant, "slug-only");
        assert_eq!(out.source, "auto");
    }

    #[test]
    fn list_count_many_needs_pick_with_candidates() {
        let now = 1_000_000;
        let candidates = vec![
            json!({ "id": "team-a", "name": "A" }),
            json!({ "id": "team-b", "name": "B" }),
        ];
        let list = {
            let candidates = candidates.clone();
            move || Some(candidates)
        };
        let out = resolve_with(None, now, TTL, list, no_team);
        assert_eq!(out.tenant, "");
        assert_eq!(out.source, "list");
        assert!(out.needs_pick);
        assert_eq!(out.candidates, candidates);
    }

    #[test]
    fn empty_list_with_team_id_uses_preflight_fallback() {
        let now = 1_000_000;
        let list = || Some(Vec::new());
        let team = || Some("team-preflight".to_string());
        let out = resolve_with(None, now, TTL, list, team);
        assert_eq!(out.tenant, "team-preflight");
        assert_eq!(out.source, "preflight");
        assert!(!out.needs_pick);
        assert!(out.candidates.is_empty());
    }

    #[test]
    fn list_none_with_team_id_uses_preflight_fallback() {
        let now = 1_000_000;
        let team = || Some("team-preflight".to_string());
        let out = resolve_with(None, now, TTL, no_list, team);
        assert_eq!(out.tenant, "team-preflight");
        assert_eq!(out.source, "preflight");
    }

    #[test]
    fn everything_empty_yields_empty_output() {
        let now = 1_000_000;
        let out = resolve_with(None, now, TTL, no_list, no_team);
        assert_eq!(out.tenant, "");
        assert_eq!(out.source, "");
        assert!(!out.needs_pick);
        assert!(out.candidates.is_empty());
    }

    #[test]
    fn empty_team_id_yields_empty_output() {
        let now = 1_000_000;
        let team = || Some(String::new());
        let out = resolve_with(None, now, TTL, no_list, team);
        assert_eq!(out.tenant, "");
        assert_eq!(out.source, "");
    }
}

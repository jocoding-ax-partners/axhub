use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::axhub_cli::{run_axhub, CliOutput, DEFAULT_AXHUB_TIMEOUT};
use crate::cli_envelope::{
    envelope_status, error_code, error_message, looks_like_error_envelope, parse_json_stdout, rows,
    status_string, string_at_any, unwrap_data,
};
use crate::redact::redact;

pub const DEFAULT_LIMIT: usize = 5;
pub const EXIT_LIST_OK: i32 = 0;
pub const EXIT_LIST_AUTH: i32 = 65;
pub const EXIT_LIST_NOT_FOUND: i32 = 67;
pub const EXIT_LIST_TRANSPORT: i32 = 1;

// Compile-time pins: the helper OUTPUT exit-namespace values are a stable
// contract consumed by 30+ downstream skills — they must not silently drift.
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(EXIT_LIST_AUTH == 65);
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(EXIT_LIST_NOT_FOUND == 67);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentSummary {
    pub id: String,
    pub app_id: String,
    pub status: String,
    pub commit_sha: String,
    pub commit_message: String,
    pub branch: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDeploymentsArgs {
    /// Legacy public name retained for CLI/API compatibility. Internally this is
    /// a string app reference accepted by current `axhub deploy list --app`:
    /// slug, UUID, or any future app ref the canonical CLI understands.
    pub app_id: String,
    pub limit: Option<usize>,
}

/// Current wire schema version for `ListDeploymentsResult`. Bumped to `2`
/// in PR #149 when:
/// - `endpoint_used` collapsed from a resolved URL to the literal `"cli"`.
/// - `DeploymentSummary.id` / `app_id` changed from `i64` to `String`.
/// - `DeploymentSummary.created_at` was normalized to RFC3339 + millis.
///
/// Consumers can branch on `schema_version` to migrate safely; pre-PR
/// payloads (no field) deserialize as `1` via [`default_schema_version`].
pub const LIST_DEPLOYMENTS_SCHEMA_VERSION: u32 = 2;

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListDeploymentsResult {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub deployments: Vec<DeploymentSummary>,
    pub endpoint_used: String,
    pub exit_code: i32,
    pub error_code: Option<String>,
    pub error_message_kr: Option<String>,
}

/// Default window for "recently pushed" classification (separate from
/// `recovery_scan::DEFAULT_STALE_THRESHOLD_SECS` even though the numeric
/// value is the same — the two thresholds serve different purposes).
pub const RECENTLY_PUSHED_WINDOW_SECS: u64 = 60;

/// Statuses that indicate a deploy is actively in progress.
const IN_FLIGHT_STATUSES: &[&str] = &[
    "pending",
    "queued",
    "building",
    "deploying",
    "running",
    "in_progress",
];

/// A deploy that is currently in-flight for a given app.
///
/// JSON shape: `{"id": "<str>", "status": "<str>", "created_at": "<RFC3339>", "commit_sha": "<str>"}`.
/// `seconds_since_created` is kept for internal computation (saturating_sub
/// clock-skew guard) but excluded from the public JSON envelope — the SKILL
/// layer uses shell `date` arithmetic for deterministic timing comparisons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InFlightDeploy {
    pub id: String,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub commit_sha: String,
    #[serde(skip)]
    pub seconds_since_created: u64,
}

/// Maximum app reference length accepted on the helper boundary. Aligns
/// with backend app-slug constraints (≤64 chars per `axhub` v0.15) and
/// gives a hard upper bound so a hostile argv cannot blow up downstream.
pub const APP_REF_MAX_LEN: usize = 64;

/// Validate an app reference (slug, UUID, or other future identifier) before
/// it is forwarded to the canonical `axhub` CLI as an argv element.
///
/// Rejects values that:
/// - are empty or longer than [`APP_REF_MAX_LEN`]
/// - start with `-` (would be interpreted as a flag by clap)
/// - contain characters outside `[A-Za-z0-9_-]`
///
/// `Command::new(...).arg(...)` already prevents *shell* injection, but
/// flag-shaped values (`--malicious`, `-h`) and embedded whitespace can
/// still confuse the downstream parser. Fail fast here so probes never
/// spawn a subprocess with a hostile argv.
pub fn validate_app_ref(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("app reference is empty".to_string());
    }
    if value.len() > APP_REF_MAX_LEN {
        return Err(format!(
            "app reference exceeds {APP_REF_MAX_LEN} chars (got {})",
            value.len()
        ));
    }
    if value.starts_with('-') {
        return Err("app reference must not start with '-'".to_string());
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("app reference may only contain [A-Za-z0-9_-]".to_string());
    }
    Ok(())
}

pub fn list_deployments_cli_args(args: &ListDeploymentsArgs) -> Vec<String> {
    let limit = args
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, 100)
        .to_string();
    vec![
        "--json".into(),
        "deploy".into(),
        "list".into(),
        "--app".into(),
        args.app_id.clone(),
        "--page-size".into(),
        limit,
    ]
}

pub fn run_list_deployments(args: ListDeploymentsArgs) -> ListDeploymentsResult {
    run_list_deployments_with_runner(args, |argv| {
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        run_axhub(&refs)
    })
}

pub fn run_list_deployments_with_runner<F>(
    args: ListDeploymentsArgs,
    runner: F,
) -> ListDeploymentsResult
where
    F: Fn(&[String]) -> CliOutput,
{
    if let Err(reason) = validate_app_ref(&args.app_id) {
        return ListDeploymentsResult {
            schema_version: LIST_DEPLOYMENTS_SCHEMA_VERSION,
            deployments: vec![],
            endpoint_used: "cli".into(),
            exit_code: EXIT_LIST_TRANSPORT,
            error_code: Some("validation.app_id_invalid".into()),
            error_message_kr: Some(format!("app 식별자가 올바르지 않아요: {reason}")),
        };
    }
    let argv = list_deployments_cli_args(&args);
    let output = runner(&argv);
    parse_list_deployments_cli_output(&args, output)
}

pub fn parse_list_deployments_cli_output(
    args: &ListDeploymentsArgs,
    output: CliOutput,
) -> ListDeploymentsResult {
    if output.timed_out {
        return transport_error("transport.timeout", "axhub deploy list timeout (5초)");
    }
    if output.exit_code == 127 {
        return transport_error(
            "transport.cli_missing",
            "axhub CLI 가 PATH 에 없거나 실행할 수 없어요. `axhub --version` 으로 확인하거나 axhub:setup 으로 다시 설치해주세요.",
        );
    }

    let parsed = match parse_json_stdout(&output.stdout) {
        Ok(value) => value,
        Err(err) => {
            let code = exit_to_error_code(output.exit_code, None);
            if output.exit_code == EXIT_LIST_OK {
                // exit 0 + parse failure: distinguish "CLI returned an
                // error-shaped envelope using a field shape we don't
                // recognize yet" from "wire-level garbage" so SKILL
                // routing isn't forced to retry transport errors that
                // are actually domain-level. PR #149 / review #12.
                let code = if looks_like_error_envelope(&output.stdout) {
                    "response.error_envelope_unknown_shape"
                } else {
                    "response.invalid_json"
                };
                return transport_error(
                    code,
                    &format!("axhub deploy list 응답 파싱 실패. ({err})"),
                );
            }
            return ListDeploymentsResult {
                schema_version: LIST_DEPLOYMENTS_SCHEMA_VERSION,
                deployments: vec![],
                endpoint_used: "cli".into(),
                exit_code: exit_to_helper_exit(output.exit_code, code.as_deref()),
                error_code: Some(code.unwrap_or_else(|| "response.invalid_json".into())),
                error_message_kr: Some(stderr_or_generic(&output.stderr)),
            };
        }
    };

    if output.exit_code != 0 || envelope_status(&parsed) == Some("error") {
        let code = error_code(&parsed).or_else(|| exit_to_error_code(output.exit_code, None));
        return ListDeploymentsResult {
            schema_version: LIST_DEPLOYMENTS_SCHEMA_VERSION,
            deployments: vec![],
            endpoint_used: "cli".into(),
            exit_code: exit_to_helper_exit(output.exit_code, code.as_deref()),
            error_code: code,
            error_message_kr: Some(
                error_message(&parsed).unwrap_or_else(|| stderr_or_generic(&output.stderr)),
            ),
        };
    }

    let deployments = rows(&parsed)
        .iter()
        .filter_map(|row| deployment_summary_from_value(row, &args.app_id))
        .collect::<Vec<_>>();

    ListDeploymentsResult {
        schema_version: LIST_DEPLOYMENTS_SCHEMA_VERSION,
        deployments,
        endpoint_used: "cli".into(),
        exit_code: EXIT_LIST_OK,
        error_code: None,
        error_message_kr: None,
    }
}

pub fn find_app_in_flight_with_runner<F>(
    app_ref: &str,
    now: chrono::DateTime<chrono::Utc>,
    window_secs: u64,
    runner: F,
) -> Result<Option<InFlightDeploy>, anyhow::Error>
where
    F: Fn(&[String]) -> CliOutput,
{
    let result = run_list_deployments_with_runner(
        ListDeploymentsArgs {
            app_id: app_ref.to_string(),
            limit: Some(DEFAULT_LIMIT),
        },
        runner,
    );
    in_flight_from_list_result(result, now, window_secs)
}

pub fn find_app_in_flight_with_window(
    app_ref: &str,
    now: chrono::DateTime<chrono::Utc>,
    window_secs: u64,
) -> Result<Option<InFlightDeploy>, anyhow::Error> {
    find_app_in_flight_with_runner(app_ref, now, window_secs, |argv| {
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        crate::axhub_cli::run_axhub_with_timeout(
            &crate::axhub_cli::axhub_bin_from_env(),
            &refs,
            DEFAULT_AXHUB_TIMEOUT,
        )
    })
}

fn in_flight_from_list_result(
    result: ListDeploymentsResult,
    now: chrono::DateTime<chrono::Utc>,
    window_secs: u64,
) -> Result<Option<InFlightDeploy>, anyhow::Error> {
    if result.exit_code != EXIT_LIST_OK {
        return Err(anyhow::anyhow!(result
            .error_message_kr
            .unwrap_or_else(|| "list_deployments failed".into())));
    }

    let now_secs = now.timestamp().max(0) as u64;

    for d in result.deployments {
        if !IN_FLIGHT_STATUSES
            .iter()
            .any(|status| status.eq_ignore_ascii_case(&d.status))
        {
            continue;
        }
        let created_dt = chrono::DateTime::parse_from_rfc3339(&d.created_at)
            .map_err(|e| anyhow::anyhow!("created_at parse failed: {e}"))?
            .with_timezone(&chrono::Utc);
        let created_secs = created_dt.timestamp().max(0) as u64;
        let seconds_since_created = now_secs.saturating_sub(created_secs);
        if seconds_since_created <= window_secs {
            let canonical = created_dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            return Ok(Some(InFlightDeploy {
                id: d.id,
                status: d.status,
                created_at: canonical,
                commit_sha: d.commit_sha,
                seconds_since_created,
            }));
        }
    }

    Ok(None)
}

fn deployment_summary_from_value(value: &Value, fallback_app: &str) -> Option<DeploymentSummary> {
    let value = unwrap_data(value);
    let id = string_at_any(value, &["id", "deployment_id", "deploy_id"])?;
    let app_id = string_at_any(value, &["app_id", "appId", "app", "app_slug"])
        .unwrap_or_else(|| fallback_app.to_string());
    let status = status_string(value).unwrap_or_else(|| "unknown".into());
    let commit_sha = string_at_any(value, &["commit_sha", "commit", "sha"]).unwrap_or_default();
    let commit_message = string_at_any(value, &["commit_message", "message"]).unwrap_or_default();
    let branch = string_at_any(value, &["branch", "git_branch"]).unwrap_or_default();
    let created_at = string_at_any(
        value,
        &[
            "created_at",
            "createdAt",
            "started_at",
            "startedAt",
            "completed_at",
            "completedAt",
        ],
    )
    .unwrap_or_default();

    Some(DeploymentSummary {
        id,
        app_id,
        status,
        commit_sha,
        commit_message,
        branch,
        created_at,
    })
}

fn exit_to_error_code(exit_code: i32, parsed_code: Option<&str>) -> Option<String> {
    if let Some(code) = parsed_code {
        return Some(code.to_string());
    }
    // Map the spawned `axhub` CLI's exit code to the flat closed-enum
    // `error.code` it emits (auth, not_found, usage). Current CLI contract:
    // 4=unauth, 5=not_found, 64=usage. 124/127 are shell-level (timeout /
    // command-missing), not CLI codes.
    match exit_code {
        0 => None,
        4 => Some("auth".into()),
        5 => Some("not_found".into()),
        64 => Some("usage".into()),
        124 => Some("transport.timeout".into()),
        127 => Some("transport.cli_missing".into()),
        code => Some(format!("cli.exit_{code}")),
    }
}

fn exit_to_helper_exit(exit_code: i32, code: Option<&str>) -> i32 {
    // OUTPUT namespace stays the helper's own (EXIT_LIST_AUTH=65 /
    // EXIT_LIST_NOT_FOUND=67); only the INPUT side reads the current CLI
    // contract: exit 4 / code "auth" -> auth, exit 5 / code "not_found".
    match code.unwrap_or_default() {
        c if c == "auth" || exit_code == 4 => EXIT_LIST_AUTH,
        c if c.contains("not_found") || exit_code == 5 => EXIT_LIST_NOT_FOUND,
        _ => EXIT_LIST_TRANSPORT,
    }
}

fn transport_error(code: &str, message: &str) -> ListDeploymentsResult {
    ListDeploymentsResult {
        schema_version: LIST_DEPLOYMENTS_SCHEMA_VERSION,
        deployments: vec![],
        endpoint_used: "cli".into(),
        exit_code: EXIT_LIST_TRANSPORT,
        error_code: Some(code.into()),
        error_message_kr: Some(message.into()),
    }
}

const GENERIC_LIST_FAILURE_MSG: &str = "axhub deploy list 실행에 실패했어요.";

/// Surface a user-visible message for upstream-error / parse-failure paths.
///
/// Raw `axhub` stderr is run through [`redact`] before being shown so an
/// upstream CLI that leaks credentials in trace output cannot reach
/// `systemMessage` or hook telemetry verbatim. Falls back to a generic
/// Korean message when redacted stderr is empty or all-whitespace.
fn stderr_or_generic(stderr: &str) -> String {
    let cleaned = redact(stderr);
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        GENERIC_LIST_FAILURE_MSG.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_ok(body: impl Into<String>) -> CliOutput {
        CliOutput {
            stdout: body.into(),
            stderr: String::new(),
            exit_code: 0,
            timed_out: false,
        }
    }

    fn cli_err(code: i32, body: impl Into<String>) -> CliOutput {
        CliOutput {
            stdout: body.into(),
            stderr: String::new(),
            exit_code: code,
            timed_out: false,
        }
    }

    fn deployment_json(id: impl Into<String>, status: impl Into<Value>, created_at: &str) -> Value {
        serde_json::json!({
            "id": id.into(),
            "app_id": "app_42",
            "status": status.into(),
            "commit_sha": "deadbeef",
            "commit_message": null,
            "branch": null,
            "started_at": created_at
        })
    }

    #[test]
    fn builds_current_cli_deploy_list_args() {
        let args = ListDeploymentsArgs {
            app_id: "paydrop".into(),
            limit: Some(3),
        };
        assert_eq!(
            list_deployments_cli_args(&args),
            vec![
                "--json",
                "deploy",
                "list",
                "--app",
                "paydrop",
                "--page-size",
                "3"
            ]
        );
    }

    #[test]
    fn parses_current_cli_items_shape_with_string_ids() {
        let result = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "paydrop".into(),
                limit: Some(1),
            },
            cli_ok(
                r#"{"items":[{"id":"dep_7","app_id":"app_uuid","status":"running","commit_sha":"abc","started_at":"2026-04-29T00:00:00Z"}]}"#,
            ),
        );
        assert_eq!(result.exit_code, EXIT_LIST_OK);
        assert_eq!(result.endpoint_used, "cli");
        assert_eq!(result.deployments[0].id, "dep_7");
        assert_eq!(result.deployments[0].app_id, "app_uuid");
        assert_eq!(result.deployments[0].status, "running");
        assert_eq!(result.deployments[0].created_at, "2026-04-29T00:00:00Z");
    }

    #[test]
    fn parses_enveloped_data_items_shape() {
        let result = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "paydrop".into(),
                limit: None,
            },
            cli_ok(
                r#"{"schema_version":"1","status":"ok","data":{"items":[{"id":"dep_1","status":"succeeded","started_at":"2026-04-29T00:00:00Z"}]}}"#,
            ),
        );
        assert_eq!(result.exit_code, EXIT_LIST_OK);
        assert_eq!(result.deployments[0].app_id, "paydrop");
    }

    #[test]
    fn maps_cli_auth_and_not_found_errors() {
        // Current CLI contract: unauth -> exit 4 + flat `error.code:"auth"`,
        // not-found -> exit 5 + `error.code:"not_found"`. The helper OUTPUT
        // namespace stays EXIT_LIST_AUTH(65) / EXIT_LIST_NOT_FOUND(67).
        let auth = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "paydrop".into(),
                limit: None,
            },
            cli_err(
                4,
                r#"{"schema_version":"1","status":"error","error":{"code":"auth","hint":"login"}}"#,
            ),
        );
        assert_eq!(auth.exit_code, EXIT_LIST_AUTH);
        assert_eq!(auth.error_code.as_deref(), Some("auth"));

        let missing = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "missing".into(),
                limit: None,
            },
            cli_err(
                5,
                r#"{"schema_version":"1","status":"error","error":{"code":"not_found"}}"#,
            ),
        );
        assert_eq!(missing.exit_code, EXIT_LIST_NOT_FOUND);
        assert_eq!(missing.error_code.as_deref(), Some("not_found"));
    }

    #[test]
    fn invalid_json_response_body_returns_transport() {
        let got = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "paydrop".into(),
                limit: None,
            },
            cli_ok("not json"),
        );
        assert_eq!(got.exit_code, EXIT_LIST_TRANSPORT);
        assert_eq!(got.error_code.as_deref(), Some("response.invalid_json"));
    }

    #[test]
    fn filters_status_pending_building_deploying_only() {
        let now = chrono::Utc::now();
        let recent = (now - chrono::Duration::seconds(30)).to_rfc3339();
        let body = serde_json::json!({
            "items": [
                deployment_json("dep_active", "succeeded", &recent),
                deployment_json("dep_failed", "failed", &recent),
                deployment_json("dep_stopped", "stopped", &recent)
            ]
        })
        .to_string();

        let result =
            find_app_in_flight_with_runner("paydrop", now, 600, move |_argv| cli_ok(body.clone()))
                .unwrap();

        assert!(result.is_none(), "terminal statuses must be excluded");
    }

    #[test]
    fn filters_outside_window() {
        let now = chrono::Utc::now();
        let old = (now - chrono::Duration::seconds(700)).to_rfc3339();
        let body = serde_json::json!({ "items": [deployment_json("dep_old", "pending", &old)] })
            .to_string();

        let result =
            find_app_in_flight_with_runner("paydrop", now, 600, move |_argv| cli_ok(body.clone()))
                .unwrap();

        assert!(result.is_none(), "deploy outside window must be excluded");
    }

    #[test]
    fn returns_some_for_in_flight_within_window() {
        let now = chrono::Utc::now();
        let recent = (now - chrono::Duration::seconds(30)).to_rfc3339();
        let body = serde_json::json!({ "items": [deployment_json("dep_99", "building", &recent)] })
            .to_string();

        let result =
            find_app_in_flight_with_runner("paydrop", now, 600, move |_argv| cli_ok(body.clone()))
                .unwrap()
                .expect("expected Some for in-flight deploy");

        assert_eq!(result.id, "dep_99");
        assert_eq!(result.status, "building");
        assert!(result.created_at.ends_with('Z'));
        assert!(result.seconds_since_created >= 28 && result.seconds_since_created <= 35);
    }

    #[test]
    fn malformed_created_at_returns_err() {
        let now = chrono::Utc::now();
        let body = serde_json::json!({ "items": [deployment_json("dep_7", "pending", "not-an-rfc3339-timestamp")] })
            .to_string();

        let result =
            find_app_in_flight_with_runner("paydrop", now, 600, move |_argv| cli_ok(body.clone()));

        assert!(result.is_err(), "malformed created_at must surface as Err");
    }

    #[test]
    fn stderr_credentials_never_reach_error_message_kr() {
        // Upstream-error path with token-shaped stderr: must redact, never
        // surface the raw token in user-visible error_message_kr.
        let result = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "paydrop".into(),
                limit: None,
            },
            CliOutput {
                stdout: r#"{"schema_version":"1","status":"error","error":{}}"#.into(),
                stderr: "Authorization: Bearer abcdefghij1234567890XYZ".into(),
                exit_code: 4,
                timed_out: false,
            },
        );
        let msg = result.error_message_kr.unwrap_or_default();
        assert!(
            !msg.contains("abcdefghij1234567890XYZ"),
            "raw bearer token must not appear in error_message_kr: {msg}"
        );
        assert!(
            msg.contains("Bearer ***") || msg == GENERIC_LIST_FAILURE_MSG,
            "expected redacted or generic message, got: {msg}"
        );
    }

    #[test]
    fn malformed_json_with_non_zero_exit_redacts_stderr() {
        // Parse-failure-on-non-zero-exit path: same redaction guarantee.
        let result = parse_list_deployments_cli_output(
            &ListDeploymentsArgs {
                app_id: "paydrop".into(),
                limit: None,
            },
            CliOutput {
                stdout: "not json".into(),
                stderr: "panicked at AXHUB_TOKEN=zzzzzzzzzzzzzzzzzzzzaaaaaaa".into(),
                exit_code: 4,
                timed_out: false,
            },
        );
        let msg = result.error_message_kr.unwrap_or_default();
        assert!(
            !msg.contains("zzzzzzzzzzzzzzzzzzzzaaaaaaa"),
            "raw AXHUB_TOKEN must not appear in error_message_kr: {msg}"
        );
    }

    #[test]
    fn stderr_or_generic_falls_back_when_empty() {
        assert_eq!(stderr_or_generic(""), GENERIC_LIST_FAILURE_MSG);
        assert_eq!(stderr_or_generic("   \n"), GENERIC_LIST_FAILURE_MSG);
        assert_eq!(
            stderr_or_generic("real error text").trim(),
            "real error text"
        );
    }

    #[test]
    fn validate_app_ref_accepts_canonical_slugs() {
        assert!(validate_app_ref("paydrop").is_ok());
        assert!(validate_app_ref("my-app_42").is_ok());
        assert!(validate_app_ref("a").is_ok());
        // 36-char UUID-shaped string (with hyphens removed → 32) — typical
        // canonical CLI identifier form.
        assert!(validate_app_ref("1234567890abcdef1234567890abcdef").is_ok());
    }

    #[test]
    fn validate_app_ref_rejects_argv_injection_shapes() {
        assert!(validate_app_ref("").is_err());
        assert!(validate_app_ref("--malicious").is_err());
        assert!(validate_app_ref("-h").is_err());
        assert!(validate_app_ref("app with space").is_err());
        assert!(validate_app_ref("app;rm -rf /").is_err());
        assert!(validate_app_ref("app$(echo pwn)").is_err());
        assert!(validate_app_ref(&"a".repeat(65)).is_err());
    }

    #[test]
    fn flag_shaped_app_id_never_spawns_subprocess() {
        let spawn_count = std::sync::atomic::AtomicUsize::new(0);
        let result = run_list_deployments_with_runner(
            ListDeploymentsArgs {
                app_id: "--malicious".into(),
                limit: None,
            },
            |_argv| {
                spawn_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                cli_ok("{}")
            },
        );
        assert_eq!(spawn_count.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(result.exit_code, EXIT_LIST_TRANSPORT);
        assert_eq!(
            result.error_code.as_deref(),
            Some("validation.app_id_invalid")
        );
    }
}

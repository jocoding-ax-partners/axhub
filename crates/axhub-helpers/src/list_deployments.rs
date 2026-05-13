use std::fs;
#[cfg(not(coverage))]
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
#[cfg(not(coverage))]
use std::sync::Arc;
#[cfg(not(coverage))]
use std::time::Duration;

use base64::Engine;
#[cfg(not(coverage))]
use rustls::pki_types::ServerName;
#[cfg(not(coverage))]
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DEFAULT_ENDPOINT: &str = "https://hub-api.jocodingax.ai";
pub const HUB_API_HOST: &str = "hub-api.jocodingax.ai";
pub const DEFAULT_LIMIT: usize = 5;
pub const TLS_PIN_TIMEOUT_MS: u64 = 5_000;
pub const HUB_API_SPKI_SHA256_PINS: &[&str] =
    &["sha256/vmsW4ExrgK3t3mFNtwk6KMsokm6PM+WNgC/KWhe7Z7g="];
pub const EXIT_LIST_OK: i32 = 0;
pub const EXIT_LIST_AUTH: i32 = 65;
pub const EXIT_LIST_NOT_FOUND: i32 = 67;
pub const EXIT_LIST_TRANSPORT: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentSummary {
    pub id: i64,
    pub app_id: i64,
    pub status: String,
    pub commit_sha: String,
    pub commit_message: String,
    pub branch: String,
    pub created_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDeploymentsArgs {
    pub app_id: String,
    pub limit: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListDeploymentsResult {
    pub deployments: Vec<DeploymentSummary>,
    pub endpoint_used: String,
    pub exit_code: i32,
    pub error_code: Option<String>,
    pub error_message_kr: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
pub struct TlsPinError {
    pub message: String,
    pub code: String,
}
impl TlsPinError {
    pub fn new(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}
impl HttpResponse {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn token_from_env() -> Option<String> {
    std::env::var("AXHUB_TOKEN").ok().filter(|s| !s.is_empty())
}
fn token_from_file() -> Option<String> {
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("axhub-plugin");
    fs::read_to_string(dir.join("token"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
pub fn resolve_token() -> Option<String> {
    token_from_env().or_else(token_from_file)
}
pub fn resolve_endpoint() -> String {
    std::env::var("AXHUB_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_ENDPOINT.into())
}
pub fn proxy_override_enabled() -> bool {
    std::env::var("AXHUB_ALLOW_PROXY").as_deref() == Ok("1")
}

pub fn pinned_hub_api_url(endpoint: &str) -> Result<Option<reqwest::Url>, TlsPinError> {
    let url = reqwest::Url::parse(endpoint).map_err(|_| {
        TlsPinError::new(
            format!("잘못된 AXHUB_ENDPOINT 값이에요: {endpoint}"),
            "security.endpoint_invalid",
        )
    })?;
    if url.host_str() != Some(HUB_API_HOST) {
        return Ok(None);
    }
    if url.scheme() != "https" {
        return Err(TlsPinError::new(
            "hub-api.jocodingax.ai 는 HTTPS 로만 호출해야 해요.",
            "security.tls_required",
        ));
    }
    Ok(Some(url))
}

pub fn spki_hash_from_cert_der(raw: &[u8]) -> anyhow::Result<String> {
    let (_, cert) = x509_parser::parse_x509_certificate(raw)
        .map_err(|e| anyhow::anyhow!("x509 parse failed: {e}"))?;
    let spki = cert.tbs_certificate.subject_pki.raw;
    Ok(format!(
        "sha256/{}",
        base64::engine::general_purpose::STANDARD.encode(Sha256::digest(spki))
    ))
}

#[cfg(coverage)]
pub fn verify_hub_api_tls_pin(endpoint: &str) -> Result<(), TlsPinError> {
    if proxy_override_enabled() {
        return Ok(());
    }
    let Some(_url) = pinned_hub_api_url(endpoint)? else {
        return Ok(());
    };
    Err(TlsPinError::new(
        "coverage build skips live TLS socket validation",
        "security.tls_pin_failed",
    ))
}

#[cfg(not(coverage))]
pub fn verify_hub_api_tls_pin(endpoint: &str) -> Result<(), TlsPinError> {
    if proxy_override_enabled() {
        return Ok(());
    }
    let Some(url) = pinned_hub_api_url(endpoint)? else {
        return Ok(());
    };
    let host = url.host_str().unwrap_or(HUB_API_HOST).to_string();
    let port = url.port().unwrap_or(443);
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| {
            TlsPinError::new(
                format!("hub-api TLS 연결에 실패했어요: {e}"),
                "security.tls_pin_failed",
            )
        })?
        .next()
        .ok_or_else(|| {
            TlsPinError::new(
                "hub-api TLS 주소를 찾을 수 없어요.",
                "security.tls_pin_failed",
            )
        })?;
    let mut sock = TcpStream::connect_timeout(&addr, Duration::from_millis(TLS_PIN_TIMEOUT_MS))
        .map_err(|e| {
            TlsPinError::new(
                format!("hub-api TLS 연결에 실패했어요: {e}"),
                "security.tls_pin_failed",
            )
        })?;
    sock.set_read_timeout(Some(Duration::from_millis(TLS_PIN_TIMEOUT_MS)))
        .ok();
    sock.set_write_timeout(Some(Duration::from_millis(TLS_PIN_TIMEOUT_MS)))
        .ok();
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    );
    let server_name = ServerName::try_from(host.clone()).map_err(|_| {
        TlsPinError::new(
            "hub-api TLS 서버 이름이 올바르지 않아요.",
            "security.endpoint_invalid",
        )
    })?;
    let mut conn = ClientConnection::new(config, server_name).map_err(|e| {
        TlsPinError::new(
            format!("hub-api TLS 설정에 실패했어요: {e}"),
            "security.tls_pin_failed",
        )
    })?;
    while conn.is_handshaking() {
        conn.complete_io(&mut sock).map_err(|e| {
            TlsPinError::new(
                format!("hub-api TLS pin 검증에 실패했어요: {e}"),
                "security.tls_pin_failed",
            )
        })?;
    }
    let certs = conn.peer_certificates().ok_or_else(|| {
        TlsPinError::new(
            "hub-api 인증서 원본을 읽을 수 없어요.",
            "security.tls_pin_failed",
        )
    })?;
    let leaf = certs.first().ok_or_else(|| {
        TlsPinError::new(
            "hub-api 인증서 원본을 읽을 수 없어요.",
            "security.tls_pin_failed",
        )
    })?;
    let actual = spki_hash_from_cert_der(leaf.as_ref()).map_err(|e| {
        TlsPinError::new(
            format!("hub-api TLS pin 검증에 실패했어요: {e}"),
            "security.tls_pin_failed",
        )
    })?;
    if HUB_API_SPKI_SHA256_PINS.contains(&actual.as_str()) {
        Ok(())
    } else {
        Err(TlsPinError::new(
            format!("hub-api TLS pin mismatch: {actual}"),
            "security.tls_pin_failed",
        ))
    }
}

fn parse_app_id(raw: &str) -> Option<i64> {
    raw.parse::<i64>().ok().filter(|n| *n > 0)
}
fn status_name(status: i64) -> String {
    match status {
        0 => "pending",
        1 => "building",
        2 => "deploying",
        3 => "active",
        4 => "failed",
        5 => "stopped",
        _ => return format!("unknown_{status}"),
    }
    .to_string()
}
fn build_auth_error() -> ListDeploymentsResult {
    ListDeploymentsResult { deployments: vec![], endpoint_used: resolve_endpoint(), exit_code: EXIT_LIST_AUTH, error_code: Some("auth.token_missing".into()), error_message_kr: Some("axhub 토큰을 찾을 수 없어요. 한 번만 로그인하시면 다음부터는 자동으로 작동해요:\n  axhub auth login\n또는 환경변수 우회: export AXHUB_TOKEN=axhub_pat_...".into()) }
}

#[derive(Debug, Deserialize)]
struct BackendDeployment {
    id: i64,
    app_id: i64,
    status: i64,
    commit_sha: String,
    commit_message: Option<String>,
    branch: Option<String>,
    created_at: String,
}
#[derive(Debug, Deserialize)]
struct BackendData {
    deployments: Option<Vec<BackendDeployment>>,
}
#[derive(Debug, Deserialize)]
struct BackendListEnvelope {
    data: Option<BackendData>,
}

pub fn run_list_deployments_with_fetch<F, T>(
    args: ListDeploymentsArgs,
    fetch_fn: F,
    tls_pin_checker: Option<T>,
) -> ListDeploymentsResult
where
    F: Fn(&str, &str) -> Result<HttpResponse, anyhow::Error>,
    T: Fn(&str) -> Result<(), TlsPinError>,
{
    let endpoint = resolve_endpoint();
    let token = match resolve_token() {
        Some(t) => t,
        None => return build_auth_error(),
    };
    let app_id = match parse_app_id(&args.app_id) { Some(id) => id, None => return ListDeploymentsResult { deployments: vec![], endpoint_used: endpoint, exit_code: EXIT_LIST_NOT_FOUND, error_code: Some("validation.app_id_invalid".into()), error_message_kr: Some(format!("앱 ID '{}' 형식이 잘못됐어요. 숫자만 입력해주세요. (slug 입력 시 axhub apps list 로 ID 확인)", args.app_id)) } };
    let limit = args.limit.unwrap_or(DEFAULT_LIMIT);
    let url = format!("{endpoint}/api/v1/apps/{app_id}/deployments?per_page={limit}");
    let resp = match (|| -> Result<HttpResponse, anyhow::Error> {
        if let Some(checker) = tls_pin_checker.as_ref() {
            checker(&endpoint).map_err(|e| anyhow::anyhow!(e))?;
        }
        fetch_fn(&url, &token)
    })() {
        Ok(r) => r,
        Err(e) => {
            return ListDeploymentsResult {
                deployments: vec![],
                endpoint_used: endpoint,
                exit_code: EXIT_LIST_TRANSPORT,
                error_code: Some(if let Some(pin) = e.downcast_ref::<TlsPinError>() {
                    pin.code.clone()
                } else {
                    "transport.network_error".into()
                }),
                error_message_kr: Some(format!(
                    "axhub 서버까지 연결이 끊겼어요. 네트워크 확인 후 다시 시도해주세요. ({e})"
                )),
            }
        }
    };
    match resp.status {
        401 | 403 => {
            return ListDeploymentsResult {
                deployments: vec![],
                endpoint_used: endpoint,
                exit_code: EXIT_LIST_AUTH,
                error_code: Some("auth.token_invalid".into()),
                error_message_kr: Some(
                    "토큰이 만료되었거나 권한이 없어요. axhub auth login 으로 다시 인증해주세요."
                        .into(),
                ),
            }
        }
        404 => {
            return ListDeploymentsResult {
                deployments: vec![],
                endpoint_used: endpoint,
                exit_code: EXIT_LIST_NOT_FOUND,
                error_code: Some("resource.app_not_found".into()),
                error_message_kr: Some(format!(
                    "app id {app_id} 를 찾을 수 없어요. axhub apps list 로 정확한 ID 확인해주세요."
                )),
            }
        }
        s if !(200..300).contains(&s) => {
            return ListDeploymentsResult {
                deployments: vec![],
                endpoint_used: endpoint,
                exit_code: EXIT_LIST_TRANSPORT,
                error_code: Some(format!("http.{s}")),
                error_message_kr: Some(format!(
                    "서버 응답 에러 (HTTP {s}). 잠시 후 다시 시도해주세요."
                )),
            }
        }
        _ => {}
    }
    let env: BackendListEnvelope = match serde_json::from_str(&resp.body) {
        Ok(v) => v,
        Err(e) => {
            return ListDeploymentsResult {
                deployments: vec![],
                endpoint_used: endpoint,
                exit_code: EXIT_LIST_TRANSPORT,
                error_code: Some("response.invalid_json".into()),
                error_message_kr: Some(format!("응답 파싱 실패. ({e})")),
            }
        }
    };
    let deployments = env
        .data
        .and_then(|d| d.deployments)
        .unwrap_or_default()
        .into_iter()
        .map(|d| DeploymentSummary {
            id: d.id,
            app_id: d.app_id,
            status: status_name(d.status),
            commit_sha: d.commit_sha,
            commit_message: d.commit_message.unwrap_or_default(),
            branch: d.branch.unwrap_or_default(),
            created_at: d.created_at,
        })
        .collect();
    ListDeploymentsResult {
        deployments,
        endpoint_used: endpoint,
        exit_code: EXIT_LIST_OK,
        error_code: None,
        error_message_kr: None,
    }
}

#[cfg(coverage)]
pub fn run_list_deployments(args: ListDeploymentsArgs) -> ListDeploymentsResult {
    run_list_deployments_with_fetch(
        args,
        |_url, _token| {
            Ok(HttpResponse {
                status: 200,
                body: r#"{"data":{"deployments":[]}}"#.into(),
            })
        },
        Some(verify_hub_api_tls_pin),
    )
}

#[cfg(not(coverage))]
pub fn run_list_deployments(args: ListDeploymentsArgs) -> ListDeploymentsResult {
    run_list_deployments_with_fetch(
        args,
        |url, token| {
            let client = reqwest::blocking::Client::new();
            let resp = client
                .get(url)
                .bearer_auth(token)
                .header("Accept", "application/json")
                .send()?;
            let status = resp.status().as_u16();
            let body = resp.text()?;
            Ok(HttpResponse { status, body })
        },
        Some(verify_hub_api_tls_pin),
    )
}

// ── In-flight deploy detection ────────────────────────────────────────────────

/// Default window for "recently pushed" classification (separate from
/// `recovery_scan::DEFAULT_STALE_THRESHOLD_SECS` even though the numeric
/// value is the same — the two thresholds serve different purposes).
pub const RECENTLY_PUSHED_WINDOW_SECS: u64 = 60;

/// Statuses that indicate a deploy is actively in progress.
const IN_FLIGHT_STATUSES: &[&str] = &["pending", "building", "deploying"];

/// A deploy that is currently in-flight for a given app.
///
/// JSON shape: `{"id": i64, "pushed_at": "<RFC3339>"}`.
/// `seconds_since_created` is kept for internal computation (saturating_sub
/// clock-skew guard) but excluded from the public JSON envelope — the SKILL
/// layer uses shell `date` arithmetic for deterministic timing comparisons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InFlightDeploy {
    pub id: i64,
    pub status: String,
    #[serde(rename = "pushed_at")]
    pub created_at: String, // RFC3339
    #[serde(skip)]
    pub seconds_since_created: u64,
}

/// Testable core: fetches the deployment list via `fetch_fn` and returns the
/// first in-flight deploy within `window_secs`, or `None`.
///
/// - Filters status ∈ {pending, building, deploying}.
/// - Filters `created_at` within last `window_secs`.
/// - `seconds_since_created` uses `saturating_sub` to defend against clock
///   skew underflow (clock skew where created_at > now).
/// - Does NOT match on `commit_sha`.
pub fn find_app_in_flight_with_fetch<F, T>(
    app_id: i64,
    now: chrono::DateTime<chrono::Utc>,
    window_secs: u64,
    fetch_fn: F,
    tls_pin_checker: Option<T>,
) -> Result<Option<InFlightDeploy>, anyhow::Error>
where
    F: Fn(&str, &str) -> Result<HttpResponse, anyhow::Error>,
    T: Fn(&str) -> Result<(), TlsPinError>,
{
    let result = run_list_deployments_with_fetch(
        ListDeploymentsArgs {
            app_id: app_id.to_string(),
            limit: Some(DEFAULT_LIMIT),
        },
        fetch_fn,
        tls_pin_checker,
    );

    if result.exit_code != EXIT_LIST_OK {
        return Err(anyhow::anyhow!(
            result
                .error_message_kr
                .unwrap_or_else(|| "list_deployments failed".into())
        ));
    }

    let now_secs = now.timestamp().max(0) as u64;

    for d in result.deployments {
        if !IN_FLIGHT_STATUSES.contains(&d.status.as_str()) {
            continue;
        }
        let created_dt = chrono::DateTime::parse_from_rfc3339(&d.created_at)
            .map_err(|e| anyhow::anyhow!("created_at parse failed: {e}"))?
            .with_timezone(&chrono::Utc);
        let created_secs = created_dt.timestamp().max(0) as u64;
        let seconds_since_created = now_secs.saturating_sub(created_secs);
        if seconds_since_created <= window_secs {
            let canonical = created_dt
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            return Ok(Some(InFlightDeploy {
                id: d.id,
                status: d.status,
                created_at: canonical,
                seconds_since_created,
            }));
        }
    }

    Ok(None)
}

#[cfg(not(coverage))]
pub fn find_app_in_flight_with_window(
    app_id: i64,
    now: chrono::DateTime<chrono::Utc>,
    window_secs: u64,
) -> Result<Option<InFlightDeploy>, anyhow::Error> {
    find_app_in_flight_with_fetch(
        app_id,
        now,
        window_secs,
        |url, token| {
            let client = reqwest::blocking::Client::new();
            let resp = client
                .get(url)
                .bearer_auth(token)
                .header("Accept", "application/json")
                .send()?;
            let status = resp.status().as_u16();
            let body = resp.text()?;
            Ok(HttpResponse { status, body })
        },
        Some(verify_hub_api_tls_pin),
    )
}

#[cfg(coverage)]
pub fn find_app_in_flight_with_window(
    _app_id: i64,
    _now: chrono::DateTime<chrono::Utc>,
    _window_secs: u64,
) -> Result<Option<InFlightDeploy>, anyhow::Error> {
    Ok(None)
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(all(test, not(coverage)))]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn ok_pin(_: &str) -> Result<(), TlsPinError> {
        Ok(())
    }

    fn deployment_json(id: i64, status: i64, created_at: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "app_id": 42_i64,
            "status": status,
            "commit_sha": "deadbeef",
            "commit_message": null,
            "branch": null,
            "created_at": created_at
        })
    }

    fn list_response(deps: &[serde_json::Value]) -> String {
        serde_json::json!({ "data": { "deployments": deps } }).to_string()
    }

    #[test]
    fn rustls_crypto_provider_is_unambiguous_without_proxy_override() {
        let roots = rustls::RootCertStore::empty();
        let _ = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
    }

    /// Only deployments with status pending/building/deploying (0/1/2) are
    /// returned; active (3), failed (4), stopped (5) must be excluded.
    #[test]
    fn filters_status_pending_building_deploying_only() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("AXHUB_TOKEN", "test_token");
        let now = chrono::Utc::now();
        let recent = (now - chrono::Duration::seconds(30)).to_rfc3339();

        let body = list_response(&[
            deployment_json(1, 3, &recent), // active   — excluded
            deployment_json(2, 4, &recent), // failed   — excluded
            deployment_json(3, 5, &recent), // stopped  — excluded
        ]);

        let result = find_app_in_flight_with_fetch(
            42,
            now,
            600,
            move |_url, _token| Ok(HttpResponse { status: 200, body: body.clone() }),
            Some(ok_pin as fn(&str) -> Result<(), TlsPinError>),
        )
        .unwrap();

        std::env::remove_var("AXHUB_TOKEN");
        assert!(result.is_none(), "non-in-flight statuses must be excluded");
    }

    /// A pending deploy whose created_at is older than window_secs must be
    /// excluded (now - created_at > window).
    #[test]
    fn filters_outside_window() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("AXHUB_TOKEN", "test_token");
        let now = chrono::Utc::now();
        let old = (now - chrono::Duration::seconds(700)).to_rfc3339(); // 700 s > 600 s window

        let body = list_response(&[
            deployment_json(1, 0, &old), // pending but outside window
        ]);

        let result = find_app_in_flight_with_fetch(
            42,
            now,
            600,
            move |_url, _token| Ok(HttpResponse { status: 200, body: body.clone() }),
            Some(ok_pin as fn(&str) -> Result<(), TlsPinError>),
        )
        .unwrap();

        std::env::remove_var("AXHUB_TOKEN");
        assert!(result.is_none(), "deploy outside window must be excluded");
    }
}

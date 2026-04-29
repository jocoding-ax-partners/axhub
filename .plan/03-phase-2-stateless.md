# Phase 2 — Stateless 모듈 (Week 4~6)

**기간:** 2~3주
**목표:** preflight / resolve / list-deployments Rust 포팅. fetch + semver + TLS SPKI pinning.
**위험:** **매우 높음** (list-deployments TLS pin 보안)
**선행 조건:** Phase 1 완료 + Phase 0 keyring spike (DX-4) 완료

---

## 1. 모듈별 작업

### 2.1 `preflight.rs` (257 LOC TS → ~400 LOC Rust)

**TS source:** `src/axhub-helpers/preflight.ts`
**역할:** axhub-cli 버전 체크, hub 호환성, semver compare

**핵심 정정 (Eng review 발견):**
- `preflight.ts:73` 의 regex 가 prerelease/build metadata 의도적 제거. `semver::Version::parse` 로는 부족.
- `Bun.spawnSync({ cmd: ["axhub-cli", "--version"] })` 호출 — Phase 0 spawn shim 사용.
- CHANGELOG 22.x 의 "Could not resolve: 'semver'" 회귀 재발 방지 — CI 강제.

**작업:**

```rust
// crates/axhub-helpers/src/preflight.rs
use regex::Regex;
use semver::{Version, VersionReq};
use std::sync::LazyLock;
use crate::spawn::spawn_sync;
use crate::msg;

// preflight.ts:73 의 의도적 prerelease drop 복제
static VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+)\.(\d+)\.(\d+)").unwrap()  // major.minor.patch only
});

#[derive(Debug, thiserror::Error)]
pub enum PreflightError {
    #[error("axhub-cli not on PATH")]
    NotInstalled,
    #[error("cli version too old: {0}")]
    TooOld(String),
    #[error("hub incompatible: {0}")]
    HubIncompat(String),
}

pub async fn check_cli_version(min: &str) -> Result<Version, PreflightError> {
    let result = spawn_sync(&["axhub-cli", "--version"])
        .map_err(|_| PreflightError::NotInstalled)?;
    let version = parse_cli_version(&result.stdout)?;
    let req = VersionReq::parse(&format!(">={}", min)).unwrap();
    if !req.matches(&version) {
        return Err(PreflightError::TooOld(version.to_string()));
    }
    Ok(version)
}

fn parse_cli_version(output: &str) -> Result<Version, PreflightError> {
    // bug-for-bug parity: prerelease/build drop
    let cap = VERSION_RE.captures(output)
        .ok_or_else(|| PreflightError::TooOld(output.to_string()))?;
    let major = cap[1].parse().unwrap();
    let minor = cap[2].parse().unwrap();
    let patch = cap[3].parse().unwrap();
    Ok(Version::new(major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_version() {
        let v = parse_cli_version("axhub-cli 1.2.3").unwrap();
        assert_eq!(v, Version::new(1, 2, 3));
    }

    #[test]
    fn drop_prerelease() {
        // bug-for-bug parity with preflight.ts:73
        let v = parse_cli_version("axhub-cli 1.2.3-rc.1").unwrap();
        assert_eq!(v, Version::new(1, 2, 3));  // -rc.1 dropped
    }

    #[test]
    fn drop_build_metadata() {
        let v = parse_cli_version("axhub-cli 1.2.3+build.456").unwrap();
        assert_eq!(v, Version::new(1, 2, 3));
    }

    #[test]
    fn semver_resolve_check() {
        // CHANGELOG 22.x 회귀 방지
        let v = parse_cli_version("axhub-cli 0.1.23").unwrap();
        let req = VersionReq::parse(">=0.1.20").unwrap();
        assert!(req.matches(&v));
    }
}
```

**Test parity:**
- `tests/fixtures/preflight/` 의 모든 case 를 Rust integration test 로
- mock-hub fixture 재사용 (JSON 호환)

**Exit criteria:**
- [ ] `cargo test preflight` PASS
- [ ] prerelease drop bug-for-bug parity 검증 (`1.2.3-rc.1` → `1.2.3`)
- [ ] CI 에서 `cargo build --release && ./binary version` smoke 통과 (semver 회귀 방지)

---

### 2.2 `resolve.rs` (296 LOC TS → ~450 LOC Rust)

**TS source:** `src/axhub-helpers/resolve.ts`
**역할:** 앱 식별자 resolve, profile lookup, identity merge

**작업:**

```rust
// crates/axhub-helpers/src/resolve.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::msg;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_id: String,
    pub team: Option<String>,
    pub endpoint: Option<String>,
}

pub fn resolve_token() -> Option<String> {
    // env first, then file (resolve.ts 동작 복제)
    if let Ok(t) = std::env::var("AXHUB_TOKEN") {
        if !t.is_empty() { return Some(t); }
    }
    let xdg = std::env::var("XDG_CONFIG_HOME").ok()
        .filter(|s| !s.is_empty());
    let dir = match xdg {
        Some(x) => PathBuf::from(x).join("axhub-plugin"),
        None => dirs::home_dir()?.join(".config").join("axhub-plugin"),
    };
    let path = dir.join("token");
    let content = std::fs::read_to_string(&path).ok()?;
    let t = content.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

pub fn resolve_endpoint() -> String {
    std::env::var("AXHUB_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://hub-api.jocodingax.ai".to_string())
}
```

**Test parity:**
- `tests/fixtures/profiles/` 케이스 그대로
- env override 우선순위 검증
- XDG_CONFIG_HOME 동작 검증

**Exit criteria:**
- [ ] `cargo test resolve` PASS
- [ ] env vs file 우선순위 TS 동작 동일
- [ ] XDG_CONFIG_HOME 빈 문자열/missing 처리 동일

---

### 2.3 `list-deployments.rs` (339 LOC TS → ~500 LOC Rust) — **TLS PIN 핵심**

**TS source:** `src/axhub-helpers/list-deployments.ts`
**역할:** Hub API client + **TLS SPKI pinning + X509** + deployment 리스트

**핵심 정정 (Eng review 발견):**
- 이 모듈이 plan §3.3 의 "consent.rs 가 mTLS+X509" 라고 잘못 표시했던 위험의 **실제 위치**
- `HUB_API_SPKI_SHA256_PINS` const array, `X509Certificate`, `tls.connect`, `getPeerCertificate(true)` 동작 정확 보존
- `AXHUB_ALLOW_PROXY=1` 옵트아웃 보존 (corporate proxy 호환)

**의존:**

```toml
# crates/axhub-helpers/Cargo.toml
[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
rustls = "0.23"
tokio-rustls = "0.26"
x509-parser = "0.16"
sha2 = "0.10"
base64 = "0.22"
```

**작업:**

```rust
// crates/axhub-helpers/src/list_deployments.rs
use rustls::pki_types::ServerName;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::prelude::*;

const HUB_API_HOST: &str = "hub-api.jocodingax.ai";
const HUB_API_SPKI_SHA256_PINS: &[&str] = &[
    // list-deployments.ts:34 의 pin 들 정확 복사
    "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",  // ← 실제 값으로 교체
    "sha256/BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=",
];
const TLS_PIN_TIMEOUT_MS: u64 = 5000;

#[derive(Debug, thiserror::Error)]
pub enum TlsPinError {
    #[error("hub-api TLS pin 검증에 실패했어요. {0}")]
    Failed(String),
    #[error("hub-api TLS pin 검증 시간이 초과됐어요.")]
    Timeout,
    #[error("잘못된 AXHUB_ENDPOINT 값이에요: {0}")]
    InvalidEndpoint(String),
    #[error("hub-api.jocodingax.ai 는 HTTPS 로만 호출해야 해요.")]
    HttpsRequired,
}

pub async fn verify_hub_api_tls_pin(endpoint: &str) -> Result<(), TlsPinError> {
    // AXHUB_ALLOW_PROXY=1 옵트아웃
    if std::env::var("AXHUB_ALLOW_PROXY").as_deref() == Ok("1") {
        return Ok(());
    }
    
    let url = url::Url::parse(endpoint)
        .map_err(|_| TlsPinError::InvalidEndpoint(endpoint.to_string()))?;
    
    if url.host_str() != Some(HUB_API_HOST) {
        return Ok(());  // not pinned host, skip
    }
    if url.scheme() != "https" {
        return Err(TlsPinError::HttpsRequired);
    }
    
    let port = url.port().unwrap_or(443);
    let host = url.host_str().unwrap();
    
    let timeout = std::time::Duration::from_millis(TLS_PIN_TIMEOUT_MS);
    tokio::time::timeout(timeout, async {
        let stream = TcpStream::connect((host, port)).await
            .map_err(|e| TlsPinError::Failed(e.to_string()))?;
        
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        config.dangerous().set_certificate_verifier(/* custom */);
        
        let connector = TlsConnector::from(Arc::new(config));
        let server_name = ServerName::try_from(host.to_string())
            .map_err(|_| TlsPinError::InvalidEndpoint(host.to_string()))?;
        
        let tls = connector.connect(server_name, stream).await
            .map_err(|e| TlsPinError::Failed(e.to_string()))?;
        
        let (_, conn) = tls.get_ref();
        let certs = conn.peer_certificates()
            .ok_or_else(|| TlsPinError::Failed("no peer cert".into()))?;
        
        let leaf = certs.first()
            .ok_or_else(|| TlsPinError::Failed("empty cert chain".into()))?;
        
        let actual_pin = spki_hash_from_cert(leaf.as_ref())?;
        if !HUB_API_SPKI_SHA256_PINS.contains(&actual_pin.as_str()) {
            return Err(TlsPinError::Failed(format!("pin mismatch: {}", actual_pin)));
        }
        Ok(())
    })
    .await
    .map_err(|_| TlsPinError::Timeout)?
}

fn spki_hash_from_cert(raw: &[u8]) -> Result<String, TlsPinError> {
    let (_, cert) = X509Certificate::from_der(raw)
        .map_err(|e| TlsPinError::Failed(e.to_string()))?;
    let spki_der = cert.public_key().raw;
    let hash = Sha256::digest(spki_der);
    Ok(format!("sha256/{}", base64::engine::general_purpose::STANDARD.encode(hash)))
}
```

**핵심 test:**

```rust
#[tokio::test]
async fn pin_match_succeeds() {
    // mock TLS server with known cert, pin matches HUB_API_SPKI_SHA256_PINS[0]
    let server = mock_tls_server_with_known_cert().await;
    let endpoint = format!("https://{}", HUB_API_HOST);
    // ... force connect to mock ...
    assert!(verify_hub_api_tls_pin(&endpoint).await.is_ok());
}

#[tokio::test]
async fn pin_mismatch_fails() {
    // mock server with different cert
    let server = mock_tls_server_with_different_cert().await;
    let result = verify_hub_api_tls_pin(&endpoint).await;
    assert!(matches!(result, Err(TlsPinError::Failed(_))));
}

#[tokio::test]
async fn proxy_override_skips() {
    std::env::set_var("AXHUB_ALLOW_PROXY", "1");
    let result = verify_hub_api_tls_pin("https://hub-api.jocodingax.ai").await;
    assert!(result.is_ok());  // skipped
    std::env::remove_var("AXHUB_ALLOW_PROXY");
}

#[tokio::test]
async fn timeout() {
    let endpoint = "https://hub-api.jocodingax.ai:9999";  // unreachable port
    let result = verify_hub_api_tls_pin(endpoint).await;
    assert!(matches!(result, Err(TlsPinError::Timeout)));
}

#[tokio::test]
async fn https_required() {
    let result = verify_hub_api_tls_pin("http://hub-api.jocodingax.ai").await;
    assert!(matches!(result, Err(TlsPinError::HttpsRequired)));
}

#[tokio::test]
async fn non_pinned_host_skipped() {
    // 다른 호스트는 skip
    let result = verify_hub_api_tls_pin("https://example.com").await;
    assert!(result.is_ok());
}
```

**Hub API client:**

```rust
pub async fn run_list_deployments(
    token: &str,
    endpoint: &str,
) -> anyhow::Result<Vec<Deployment>> {
    verify_hub_api_tls_pin(endpoint).await?;
    
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()?;
    
    let resp = client.get(format!("{}/v1/deployments", endpoint))
        .bearer_auth(token)
        .send()
        .await?;
    
    let deps: Vec<Deployment> = resp.json().await?;
    Ok(deps)
}
```

**Exit criteria:**
- [ ] `cargo test list_deployments` 19개 case PASS
- [ ] TLS pin mismatch 시 한글 에러 메시지 (해요체) byte-equal with TS
- [ ] AXHUB_ALLOW_PROXY=1 옵트아웃 동작 검증
- [ ] timeout 5000ms 정확 (성능 회귀 없음)
- [ ] mock-hub fixture 그대로 사용 가능
- [ ] HUB_API_SPKI_SHA256_PINS 가 TS 와 byte-equal 인지 build-time assert

---

## 2. CI 추가

```yaml
- name: tls-pin-integration
  run: |
    # mock TLS server up
    bash tests/fixtures/mock-hub/start.sh &
    cargo test --test tls_pin_integration
- name: proxy-override-skip
  env:
    AXHUB_ALLOW_PROXY: "1"
  run: cargo test test_proxy_override_skips
```

---

## 3. Phase 2 Exit Criteria

- [ ] preflight / resolve / list-deployments Rust 모듈
- [ ] semver bug-for-bug parity (prerelease drop) 검증
- [ ] TLS SPKI pin 19 case 전부 PASS
- [ ] mock-hub fixture 양쪽 (TS + Rust) 동일 결과
- [ ] AXHUB_ALLOW_PROXY 옵트아웃 보존 검증
- [ ] CI 에 Rust binary smoke test 추가 (`./binary list-deployments --help`)
- [ ] CHANGELOG 22.x 의 "Could not resolve: 'semver'" 동등 회귀 차단
- [ ] 한글 에러 메시지 (해요체) lint:tone:rust PASS

---

## 4. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| TLS pin 검증 우회 (rustls custom verifier 잘못) | **치명** (MITM 가능) | `rustls::dangerous` API 사용 시 외부 페네스트 필수. ECP cert chain test |
| AXHUB_ALLOW_PROXY 동작 차이 | 높음 (사용자 차단) | env var 처리 logic byte-equal verification |
| reqwest 의 rustls-tls feature 미활성 시 OpenSSL 의존 | 중 (배포 binary 사이즈 +20MB) | Cargo.toml `default-features = false` 강제 |
| preflight semver prerelease drop 누락 | 중 (회귀) | bug-for-bug test 강제 |
| timeout 5000ms 차이 (Bun vs tokio) | 낮음 | tokio::time::timeout 정확 |

---

## 5. 다음 Phase

Phase 2 완료 시:
- list-deployments 의 TLS pin 동작 검증됨 = Hub API 직접 호출 fallback path 안전
- 다음: `04-phase-3-security.md` — consent (HMAC + parser + token file) + keychain (3 OS, EDR)
- Phase 3 가 보안 surface 의 마지막 산봉우리. security-reviewer agent + cargo-fuzz 24h 필수.

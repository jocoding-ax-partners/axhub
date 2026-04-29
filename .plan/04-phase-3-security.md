# Phase 3 — Security 모듈 (Week 7~10)

**기간:** 3~4주 (현실 추정, plan 의 2~3주 보다 +1주)
**목표:** consent (HMAC + parser + token file) + keychain (3 OS) Rust 포팅
**위험:** **매우 높음** — 보안 surface. 회귀 시 token 도용 / 한국 EDR 마찰
**선행 조건:** Phase 2 완료 + Phase 0 keyring spike (DX-4) 결과 반영

---

## 1. 핵심 정정 (Eng review 발견)

Plan 작성 시 consent.ts 가 mTLS+X509 사용한다고 잘못 표기. 실제 검증:

| Plan 주장 | 실제 source | 정정 |
|-----------|-------------|------|
| consent.ts mTLS+X509+JWE | jose HS256 only (대칭 HMAC) | 의존성: hmac+sha2+jsonwebtoken |
| 외부 페네스트 critical | parser+filesystem+HMAC lifecycle 이 실제 위험 | 위험 surface 재정의 |
| jose clockTolerance 30s | jose default 0, code 미지정 = 0 | leeway 0 lock 강제 |

**실제 보안 surface:**
1. `parseAxhubCommand` 5-level recursion (consent.ts:295-369) — Rust regex 의 linear vs JS backtracking 차이
2. Token 파일 보안 (consent.ts:151-167) — mode 0600 + O_NOFOLLOW + lstat-then-open
3. HMAC key lifecycle (consent.ts:17 주석) — `~/.local/state/axhub/hmac-key`, 32-byte CSPRNG, never logged
4. JWT exp/iat clock skew = 0 leeway
5. Binding deep-equal verification (mintConsent vs verifyLatest)

---

## 2. 모듈별 작업

### 2.1 `consent.rs` (458 LOC TS → ~700 LOC Rust)

**TS source:** `src/axhub-helpers/consent.ts`

**의존:**

```toml
[dependencies]
hmac = "0.12"
sha2 = "0.10"
jsonwebtoken = "9.3"
getrandom = "0.2"
nix = { version = "0.29", features = ["fs"] }
uuid = { version = "1", features = ["v4"] }
serde = { workspace = true }
serde_json = { workspace = true }
regex = { workspace = true }

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Storage_FileSystem",
    "Win32_Security",
] }
```

#### 2.1.1 HMAC key lifecycle

```rust
// crates/axhub-helpers/src/consent/key.rs
use std::path::PathBuf;
use std::fs;

const KEY_PATH_REL: &str = ".local/state/axhub/hmac-key";
const KEY_LEN: usize = 32;

pub fn load_or_mint_key() -> anyhow::Result<[u8; KEY_LEN]> {
    let path = key_path()?;
    
    if path.exists() {
        let bytes = read_with_lstat_check(&path)?;
        if bytes.len() != KEY_LEN {
            anyhow::bail!("HMAC key file corrupt (length mismatch)");
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }
    
    // mint new
    let mut key = [0u8; KEY_LEN];
    getrandom::getrandom(&mut key)?;
    
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        // parent dir mode 0700
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    
    write_with_o_nofollow_0600(&path, &key)?;
    Ok(key)
}

fn key_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home"))?;
    Ok(home.join(KEY_PATH_REL))
}

#[cfg(unix)]
fn write_with_o_nofollow_0600(path: &std::path::Path, data: &[u8]) -> anyhow::Result<()> {
    use nix::fcntl::{open, OFlag};
    use nix::sys::stat::Mode;
    use std::os::unix::io::FromRawFd;
    use std::io::Write;
    
    // O_CREAT | O_WRONLY | O_TRUNC | O_NOFOLLOW, mode 0600
    let fd = open(
        path,
        OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC | OFlag::O_NOFOLLOW,
        Mode::from_bits_truncate(0o600),
    )?;
    let mut f = unsafe { std::fs::File::from_raw_fd(fd) };
    f.write_all(data)?;
    Ok(())
}

#[cfg(windows)]
fn write_with_o_nofollow_0600(path: &std::path::Path, data: &[u8]) -> anyhow::Result<()> {
    // Windows: NTFS ACL — current user only
    fs::write(path, data)?;
    set_acl_user_only(path)?;
    Ok(())
}

#[cfg(unix)]
fn read_with_lstat_check(path: &std::path::Path) -> anyhow::Result<Vec<u8>> {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::symlink_metadata(path)?;  // lstat (no follow)
    if meta.file_type().is_symlink() {
        anyhow::bail!("HMAC key path is a symlink — refusing");
    }
    if meta.mode() & 0o077 != 0 {
        anyhow::bail!("HMAC key file mode too permissive: {:o}", meta.mode());
    }
    Ok(fs::read(path)?)
}
```

**핵심 test (symlink-injection 방어):**

```rust
#[test]
#[cfg(unix)]
fn refuses_symlink_target() {
    let tmp = tempdir::TempDir::new("axhub-key").unwrap();
    let key_path = tmp.path().join("hmac-key");
    let target = tmp.path().join("attacker-controlled");
    fs::write(&target, b"attacker-bytes-x32-x32-x32-xxxx").unwrap();
    std::os::unix::fs::symlink(&target, &key_path).unwrap();
    
    // O_NOFOLLOW 가 거부해야 함
    let result = read_with_lstat_check(&key_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("symlink"));
}

#[test]
#[cfg(unix)]
fn refuses_world_readable() {
    let tmp = tempdir::TempDir::new("axhub-key").unwrap();
    let key_path = tmp.path().join("hmac-key");
    fs::write(&key_path, b"x".repeat(32)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
    
    let result = read_with_lstat_check(&key_path);
    assert!(result.is_err());
}
```

#### 2.1.2 JWT mint + verify (HS256)

```rust
// crates/axhub-helpers/src/consent/jwt.rs
use jsonwebtoken::{encode, decode, Algorithm, EncodingKey, DecodingKey, Header, Validation};
use serde::{Serialize, Deserialize};
use chrono::Utc;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ConsentBinding {
    pub session_id: String,
    pub command: String,
    pub args_hash: String,
    pub minted_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ConsentClaims {
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    #[serde(flatten)]
    pub binding: ConsentBinding,
}

const TTL_SECONDS: i64 = 300;  // 5분 (consent.ts 와 동일)

pub fn mint(binding: ConsentBinding, key: &[u8; 32]) -> anyhow::Result<String> {
    let now = Utc::now().timestamp();
    let claims = ConsentClaims {
        exp: now + TTL_SECONDS,
        iat: now,
        jti: uuid::Uuid::new_v4().to_string(),
        binding,
    };
    let header = Header::new(Algorithm::HS256);
    let token = encode(&header, &claims, &EncodingKey::from_secret(key))?;
    Ok(token)
}

pub fn verify(token: &str, key: &[u8; 32], expected_binding: &ConsentBinding) -> anyhow::Result<()> {
    // CRITICAL: leeway 0 강제 (jose default 동일)
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 0;
    validation.validate_exp = true;
    validation.validate_nbf = false;
    
    let token_data = decode::<ConsentClaims>(
        token,
        &DecodingKey::from_secret(key),
        &validation,
    )?;
    
    // binding deep-equal
    if token_data.claims.binding != *expected_binding {
        anyhow::bail!("binding mismatch");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_binding() -> ConsentBinding {
        ConsentBinding {
            session_id: "s1".into(),
            command: "deploy".into(),
            args_hash: "abc".into(),
            minted_at: 1234567890,
        }
    }

    #[test]
    fn mint_verify_round_trip() {
        let key = [0u8; 32];
        let b = fixture_binding();
        let token = mint(b.clone(), &key).unwrap();
        verify(&token, &key, &b).unwrap();
    }

    #[test]
    fn rejects_expired_with_zero_leeway() {
        // CRITICAL: jose default leeway 0 동작 매칭
        let key = [0u8; 32];
        let now = Utc::now().timestamp();
        let claims = ConsentClaims {
            exp: now - 1,  // 1초 전 만료
            iat: now - 301,
            jti: "j".into(),
            binding: fixture_binding(),
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&key),
        ).unwrap();
        let result = verify(&token, &key, &fixture_binding());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expired"));
    }

    #[test]
    fn rejects_algorithm_swap() {
        // alg=none 또는 RS256 swap 시도 거부
        let key = [0u8; 32];
        let mut header = Header::new(Algorithm::HS256);
        header.alg = Algorithm::HS384;  // 다른 alg
        let claims = ConsentClaims {
            exp: Utc::now().timestamp() + 100,
            iat: Utc::now().timestamp(),
            jti: "j".into(),
            binding: fixture_binding(),
        };
        let token = encode(&header, &claims, &EncodingKey::from_secret(&key)).unwrap();
        let result = verify(&token, &key, &fixture_binding());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_binding_mismatch() {
        let key = [0u8; 32];
        let mut b = fixture_binding();
        let token = mint(b.clone(), &key).unwrap();
        b.command = "recover".into();
        let result = verify(&token, &key, &b);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("binding"));
    }
}
```

#### 2.1.3 parseAxhubCommand (parser hardening)

**TS source:** `consent.ts:295-369` (5-level recursion, regex backtracking)

**핵심 결정:** Rust `regex` crate 는 backtracking 없음 (linear time 보장). 일부 fixture 동작 다를 가능성. **bug-for-bug parity** 또는 **의도적 개선** 결정 필요.

**권장:** 의도적 개선 — explicit state machine 으로 재구현. 더 readable + linear time 보장 + fuzz 친화. 단, fixture 가 backtracking 의존했던 케이스는 결과 변경 가능 — 모든 fixture 재검증 필수.

```rust
// crates/axhub-helpers/src/consent/parser.rs
//
// parseAxhubCommand 의 의도적 재구현 (state machine).
// TS 의 regex backtracking 의존성 제거.

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub env_vars: std::collections::HashMap<String, String>,
}

pub fn parse_axhub_command(input: &str) -> Result<ParsedCommand, ParseError> {
    let mut state = ParserState::new(input);
    state.strip_wrapping_chars()?;     // quotes, parens, backticks
    state.consume_env_assignments()?;  // KEY=val 접두
    state.consume_command_token()?;
    state.consume_args()?;
    Ok(state.finish())
}

// ... state machine 구현 ...
```

**Fuzz test (cargo-fuzz, 24h 필수):**

```rust
// fuzz/fuzz_targets/parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use axhub_helpers::consent::parser::parse_axhub_command;

fuzz_target!(|input: &str| {
    // 패닉 발생 안 해야 함
    let _ = parse_axhub_command(input);
});
```

**Fuzz 실행:**

```bash
cargo fuzz run parser -- -max_total_time=86400  # 24h
```

#### 2.1.4 Consent flow (mintConsent + verifyLatest)

```rust
// crates/axhub-helpers/src/consent/mod.rs
pub mod key;
pub mod jwt;
pub mod parser;

use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::Write;

pub fn mint_consent(binding: jwt::ConsentBinding) -> anyhow::Result<String> {
    let key = key::load_or_mint_key()?;
    let token = jwt::mint(binding, &key)?;
    write_token_file(&token)?;
    Ok(token)
}

pub fn verify_latest(expected: &jwt::ConsentBinding) -> anyhow::Result<()> {
    let key = key::load_or_mint_key()?;
    let token = read_latest_token()?;
    jwt::verify(&token, &key, expected)
}

fn write_token_file(token: &str) -> anyhow::Result<()> {
    let path = token_path()?;
    // mode 0600 + O_NOFOLLOW (key 와 동일 정책)
    key::write_with_o_nofollow_0600(&path, token.as_bytes())
}
```

**Phase 3 consent.rs Exit criteria:**
- [ ] HMAC key lifecycle (load + mint + symlink defense) test PASS
- [ ] JWT mint+verify round trip + expired + alg swap + binding mismatch test PASS
- [ ] leeway=0 lock test (now-1 거부) PASS
- [ ] parser state machine 구현 + 모든 TS fixture 동등 결과 (bug-for-bug 또는 의도적 개선 명시)
- [ ] cargo-fuzz parser 24h 무결함
- [ ] symlink-as-target test 추가 PASS
- [ ] world-readable token 거부 PASS
- [ ] 한글 에러 메시지 (해요체) lint:tone:rust PASS

---

### 2.2 `keychain.rs` + `keychain_windows.rs` (354 LOC TS → ~450 LOC Rust)

**선행 조건:** Phase 0 DX-4 spike 결과 반영. keyring crate 호환되면 keyring, 아니면 subprocess.

#### Decision Tree (Phase 0 spike 결과 따라)

**Case A: keyring crate 호환 (3 OS 모두 read 가능)**

```rust
// crates/axhub-helpers/src/keychain.rs
use keyring::Entry;

pub fn read_keychain_token(service: &str, account: &str) -> anyhow::Result<String> {
    let entry = Entry::new(service, account)?;
    Ok(entry.get_password()?)
}
```

**Case B: keyring 호환 안 됨 (1+ OS 에서 read 실패)**

기존 subprocess 방식 그대로 Rust 로 port. `Bun.spawnSync` → `std::process::Command`.

```rust
// crates/axhub-helpers/src/keychain.rs (subprocess fallback)
use crate::spawn::spawn_sync;

#[cfg(target_os = "macos")]
pub fn read_keychain_token(service: &str, account: &str) -> anyhow::Result<String> {
    let result = spawn_sync(&["security", "find-generic-password", "-s", service, "-a", account, "-w"])?;
    if result.exit_code != Some(0) {
        anyhow::bail!("security CLI failed: {}", result.stderr);
    }
    Ok(strip_envelope(&result.stdout))  // go-keyring-base64: 처리
}

#[cfg(target_os = "linux")]
pub fn read_keychain_token(service: &str, account: &str) -> anyhow::Result<String> {
    let result = spawn_sync(&["secret-tool", "lookup", "service", service, "account", account])?;
    if result.exit_code != Some(0) {
        // headless fallback (Secret Service unavailable)
        return read_keychain_file_fallback(service, account);
    }
    Ok(strip_envelope(&result.stdout))
}

fn strip_envelope(raw: &str) -> String {
    // go-keyring-base64:<base64-of-JSON> 처리
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix("go-keyring-base64:") {
        // base64 decode + JSON unwrap
        // ... 
    }
    trimmed.to_string()
}
```

#### Windows (keychain_windows.rs)

**핵심 제약 (DX 발견):** 한국 EDR (V3, AhnLab, CrowdStrike) 호환. PowerShell + inline C# PInvoke 가 현재 EDR-friendly. Rust 직접 `windows-rs` 호출 시 different signature.

**권장 (DX review 따라):** PowerShell shell-out 유지. `keyring` 또는 `windows-rs` 는 EDR cohort live test (V3/AhnLab) 통과 후 채택.

```rust
// crates/axhub-helpers/src/keychain_windows.rs
use crate::spawn::spawn_sync;
use crate::msg;

const PS_SCRIPT: &str = include_str!("keychain_windows.ps1");

pub fn read_windows_keychain(service: &str, account: &str) -> anyhow::Result<String> {
    let target = format!("{}/{}", service, account);
    let result = spawn_sync(&[
        "powershell",
        "-ExecutionPolicy", "Bypass",
        "-NonInteractive",
        "-Command", PS_SCRIPT,
        "-Target", &target,
    ])?;
    
    // EDR detection (status code 0xC0000409 = STATUS_STACK_BUFFER_OVERRUN)
    if result.exit_code == Some(-1073740791i32) {
        anyhow::bail!(msg!(KEYCHAIN_EDR_BLOCKED));
    }
    
    if result.exit_code != Some(0) {
        anyhow::bail!("PowerShell exited {}: {}", result.exit_code.unwrap_or(-1), result.stderr);
    }
    
    Ok(strip_envelope(&result.stdout))
}
```

**`keychain_windows.ps1`:** `keychain-windows.ts` 의 PS_SCRIPT (53 LOC inline C#) 그대로 export.

**핵심 test:**
- V3/AhnLab cohort manual QA (자동화 어려움 — Phase 4 ship 전 실 사용자 환경에서)
- 0xC0000409 status code → EDR_BLOCKED 한글 메시지 검증
- AMSI 차단 시 fallback 메시지

**Phase 3 keychain Exit criteria:**
- [ ] DX-4 spike 결과 따른 분기 (keyring crate 또는 subprocess)
- [ ] go-keyring-base64 envelope 호환 (양방향 read/write 가능 시)
- [ ] V3/AhnLab live cohort QA 통과 (manual)
- [ ] AMSI 차단 케이스 한글 메시지 PASS
- [ ] headless Linux (Docker) fallback 동작
- [ ] `cargo test keychain` PASS

---

## 3. CI 추가

```yaml
- name: cargo-fuzz parser (24h)
  run: cargo fuzz run parser -- -max_total_time=86400
  if: github.event.schedule == '0 0 * * *'  # nightly only
- name: symlink defense
  run: cargo test --test consent_symlink_defense
- name: jwt-leeway-zero-lock
  run: cargo test rejects_expired_with_zero_leeway
- name: tone-rust
  run: bun run lint:tone:rust --strict
```

---

## 4. 보안 review 강제

Phase 3 PR 은 다음 통과 필수:
- [ ] `security-reviewer` agent (oh-my-claudecode) 통과
- [ ] `cargo-audit` clean (CVE 0건)
- [ ] `cargo-fuzz` parser 24h 무결함
- [ ] 외부 페네스트 (선택, but 권장) — solo maintainer 라 비용 ↑, 그래도 surface 가 token / HMAC 이라 1회 pen test 권장

---

## 5. Phase 3 Exit Criteria

- [ ] consent.rs (key + jwt + parser + flow) 작성
- [ ] keychain.rs (mac+linux) 작성
- [ ] keychain_windows.rs 작성 (PowerShell 유지 또는 windows-rs, DX-4 결과 따라)
- [ ] 모든 unit + integration test PASS
- [ ] cargo-fuzz parser 24h 무결함
- [ ] V3/AhnLab live cohort QA (manual) 통과
- [ ] security-reviewer agent 통과
- [ ] cargo-audit clean
- [ ] 한글 에러 메시지 lint:tone:rust PASS
- [ ] go-keyring envelope 호환 (axhub-cli 와 양방향 검증)

---

## 6. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| JWT leeway silent widen | **치명** (token replay window) | leeway=0 lock test 강제, jose default verification |
| HMAC key file permission 회귀 | **치명** (key 도난) | symlink defense test + mode 0600 lstat check |
| parser fuzz crash (regex DoS) | 높음 | cargo-fuzz 24h, state machine 의도적 재구현 |
| 한국 EDR 차단 회귀 | 매우 높음 (사용자 마찰) | PowerShell 유지 (DX-4 결과 따라) + cohort QA |
| go-keyring envelope 호환 깨짐 | 매우 높음 (axhub-cli 와 token 공유 안 됨) | 양방향 read/write test, Phase 0 spike 결과 |
| Windows ACL 보안 (token 파일) | 높음 | `windows-rs` ACL set + admin-deny test |

---

## 7. 다음 Phase

Phase 3 완료 시:
- 보안 surface 모두 Rust 화 완료
- 다음: `05-phase-4-integration.md` — main.rs CLI dispatcher + TS 제거 + v1.0.0-rust ship
- Phase 4 가 dual-runtime 종료. 사용자에게 마이그레이션 알림.

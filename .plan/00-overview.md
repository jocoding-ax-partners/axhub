# axhub-helpers TypeScript → Rust 포팅 — Overview

**작성일:** 2026-04-29
**현재 버전:** v0.1.23 (TS, Bun runtime)
**목표 버전:** v1.0.0-rust
**기간:** 10~14주 (현실 추정, /autoplan review 반영)
**담당:** Solo maintainer (`realitsyourman`)

---

## 1. 배경

axhub-helpers 는 axhub Claude Code plugin 의 helper binary. 사용자가 슬래시 커맨드 (`/axhub:deploy` 등) 호출 시 매번 실행되는 CLI. 현재 Bun runtime 으로 single binary 컴파일, 5 cross-arch (darwin-arm64/amd64, linux-arm64/amd64, windows-amd64) 배포.

### 현재 상태

- 10 모듈, 2,536 LOC (TypeScript)
- Bun 1.1+ 런타임 의존
- 외부 npm: jose 5.9.6 (HS256 JWT), semver 7.6.3, zod 3.23.8
- Test baseline: 428 `test()` + 499 `test/describe` (Phase 18)
- E2E: claude-cli matrix t1/t2/nightly
- Release: cosign 서명된 5 binary, GH release auto fire on tag push

### 포팅 동기 (사용자 결정)

사용자가 "ts로 구성된거 전부 다 rust로 완벽 포팅" 요청. /autoplan review 에서 양 모델 (Codex + Claude subagent) 모두 1주 validation sprint 권장했으나 user sovereignty 행사하여 full Rust 포팅 진행 결정.

**기대 효과 (미검증):**
- Binary 5~10x 축소 (50~90MB → 5~15MB)
- Cold start 5x 향상 (~50ms → ~10ms)
- Memory baseline -80% (~30MB → ~5MB)

**Risk acknowledged:**
- Premise (통증 driver) 미검증
- Solo maintainer + bus factor 1
- consent.rs / list-deployments.rs 보안 회귀 surface
- 8~12주 opportunity cost vs 기능 개발

---

## 2. 모듈 인벤토리 (실제 source 검증됨)

| 파일 | LOC | 역할 | 외부 의존 (실제) | 위험 |
|------|-----|------|-------------------|------|
| `index.ts` | 509 | CLI dispatcher, session-start, env shimming | Bun.spawnSync, Bun.stdin.text, fs, child_process | 높음 |
| `consent.ts` | 458 | JWT mint/verify (**HS256 only**), parser hardening, token file (mode 0600 + O_NOFOLLOW), HMAC key lifecycle | jose (HS256), crypto.randomBytes/randomUUID, fs/promises | **매우 높음** (parser+filesystem) |
| `list-deployments.ts` | 339 | Hub API client + **TLS SPKI pinning + X509** + deployment listing | fetch, **node:tls.connect**, **node:crypto.X509Certificate**, redact | **매우 높음** (TLS pin) |
| `resolve.ts` | 296 | App config resolve, identity, profile lookup | fs, env, fetch | 중 |
| `preflight.ts` | 257 | CLI version check, hub compat, semver | semver, fetch, Bun.spawnSync | 중 |
| `keychain-windows.ts` | 222 | Windows Credential Manager (PowerShell + inline C# PInvoke) | child_process (powershell) | **매우 높음** (한국 EDR 호환) |
| `catalog.ts` | 188 | NL command classifier (codegen target) | none | 낮음 |
| `keychain.ts` | 132 | macOS Keychain (security CLI) + Linux Secret Service | child_process (security/secret-tool) | 중 |
| `telemetry.ts` | 87 | Meta envelope emitter (jsonl append) | fs/promises | 낮음 |
| `redact.ts` | 48 | PII redaction utility | none | 낮음 |

**중요 정정:** 이전 plan 작성 시 consent.ts 가 mTLS+X509 사용한다고 추정했으나 grep 결과 잘못. consent.ts 는 jose HS256 (대칭 HMAC) 만. TLS+X509 는 list-deployments.ts (hub-api 직접 호출 fallback).

---

## 3. Phase 구조

| Phase | 기간 | 모듈 | 위험 | 파일 |
|-------|------|------|------|------|
| **Phase 0** | Pre-flight | DX prerequisite + spike | 중 | `01-phase-0-prerequisite.md` |
| **Phase 1** | Week 1~3 | redact, catalog, telemetry | 낮음 | `02-phase-1-foundation.md` |
| **Phase 2** | Week 4~6 | preflight, resolve, list-deployments | **매우 높음** (TLS pin) | `03-phase-2-stateless.md` |
| **Phase 3** | Week 7~10 | consent, keychain (mac+linux+win) | **매우 높음** (HMAC+EDR) | `04-phase-3-security.md` |
| **Phase 4** | Week 11~14 | main.rs entry + TS 제거 | 높음 | `05-phase-4-integration.md` |

---

## 4. 의존성 매핑 (npm → crate)

| npm | Rust crate | 비고 |
|-----|------------|------|
| `jose` ^5.9.6 (consent.ts, HS256 only) | `jsonwebtoken` 9.x | JWS-only 확인됨. josekit 불필요 |
| `semver` ^7.6.3 (preflight.ts) | `semver` 1.x + `regex` | preflight.ts:73 prerelease/build drop bug-for-bug parity 필요 |
| `zod` ^3.23.8 | `serde` + `garde` | 선언적 schema |
| `node:crypto.X509Certificate` (list-deployments.ts) | `x509-parser` + `sha2` | SPKI public key DER hash |
| `node:tls.connect` (list-deployments.ts) | `rustls` + `tokio-rustls` | hub-api leaf SPKI pinning. AXHUB_ALLOW_PROXY 보존 |
| `node:crypto.randomBytes/randomUUID` (consent.ts) | `getrandom` 직접 | rand::thread_rng 금지 |
| `node:fs constants.O_NOFOLLOW` (consent.ts) | `nix::fcntl::OFlag::O_NOFOLLOW` (Unix) + Win ACL | symlink-injection 방어 |
| `Bun.spawnSync` | `std::process::Command` | exit code + signal 정규화 shim 필요 |
| `Bun.stdin.text()` | `std::io::stdin().read_to_string()` | Windows codepage 강제 UTF-8 |
| `fetch` (Bun) | `reqwest` (rustls-tls feature) | OpenSSL 회피 |
| `bun build --compile` | `cargo build --release` matrix | Win 은 MSVC native runner |

---

## 5. 빌드/릴리스 파이프라인 변경

| 단계 | 현재 | 변경 후 |
|------|------|---------|
| Build | `bun build:all` (5 binary) | `cargo build --release` matrix (Linux/macOS = cross, Win = native MSVC) |
| Strip | (Bun 자동) | `strip` + `cargo-strip` |
| Sign | cosign sign | cosign sign (변경 없음) |
| Authenticode (Win) | signtool 가정 | signtool MSVC PE32+ (변경 없음, MSVC 강제 함의) |
| Upload | gh release upload | gh release upload (변경 없음) |
| Verify | smoke test + version assert | 동등 |

**release.yml 변경:** Bun install/compile 단계 제거, Rust toolchain setup + cargo build --release matrix 추가.

---

## 6. Test 전략

### Test 분류

| 모듈 | TS test 처리 |
|------|--------------|
| redact, telemetry, catalog | **단순 port 가능** — JSON 기반, 의존성 0 |
| preflight, resolve | **부분 port** — fixture 재사용 가능, mock 부분 재작성 |
| list-deployments | **재작성 필요** — TLS mock + bun:test spy 의존 |
| consent | **재작성 필요** — bun:test mock + parser fuzz + Korean wording assert |
| keychain (\*) | **재작성 필요** — platform-specific subprocess mock |
| main.rs (index 대응) | **재작성 필요** — clap dispatch + e2e |

**현실 비용:** 2~3주 추가 (plan 본 §5.2 가 가격 미매김)

### 회귀 방지

- consent: cargo-fuzz 24h (parser + JWT)
- list-deployments: TLS mock server 로 SPKI pin 검증
- keychain: V3/AhnLab live cohort manual QA

---

## 7. Rollback 전략

각 Phase 별 release tag 보존. 회귀 발견 시:

1. `git revert <merge-commit>` + 즉시 patch release
2. 사용자에게 `axhub update --force-version <prev>` 안내
3. cosign 서명된 이전 binary 는 GH release 에서 그대로 download
4. **Phase 1~3 동안 `AXHUB_HELPERS_RUNTIME=ts` env 로 즉시 fallback 가능 (DX-3)**
5. Phase 4 (TS 제거) 직후 1주간 monitor 의무

---

## 8. NOT in scope

- Plugin runtime (`plugin.json`, `marketplace.json`) Rust 화 — 별도 plan
- SKILL.md / nl-lexicon trigger 어구 변경
- 신규 기능 추가 (deploy/recover/update 명령어 변경)
- Bun runtime 의존성을 plugin 측에서도 제거

---

## 9. Phase 별 자세한 plan 파일

| 파일 | 내용 |
|------|------|
| `01-phase-0-prerequisite.md` | Phase 1 시작 전 8 mandatory + 2 recommended |
| `02-phase-1-foundation.md` | redact / catalog / telemetry 포팅 |
| `03-phase-2-stateless.md` | preflight / resolve / list-deployments 포팅 (TLS pin) |
| `04-phase-3-security.md` | consent (HMAC + parser + token file) + keychain (3 OS) |
| `05-phase-4-integration.md` | main.rs (CLI dispatch) + TS 제거 + v1.0.0-rust ship |
| `90-risks-and-mitigations.md` | 위험 매트릭스 + 완화책 + Cross-Phase Themes |
| `91-test-strategy.md` | 7 codepath gap + cargo-fuzz + V3/AhnLab cohort |
| `92-decision-log.md` | /autoplan dual voices + user decisions |

---

## 10. 다음 액션

1. `01-phase-0-prerequisite.md` 진행 (8 mandatory)
2. ADR 작성: `.omc/adr/0001-rust-port-decision.md`
3. Phase 0 완료 시 Phase 1 (Foundation) issue 3건 생성

**원본 review 본문:** `.omc/plans/rust-port-plan.md` (1,060줄, /autoplan 전체 결과)

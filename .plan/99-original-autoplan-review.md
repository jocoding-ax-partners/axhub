<!-- /autoplan restore point: /Users/wongil/.gstack/projects/jocoding-ax-partners-axhub/main-autoplan-restore-20260429-112948.md -->
# axhub-helpers TypeScript → Rust 완전 포팅 계획

**작성일:** 2026-04-29
**대상:** `src/axhub-helpers/` (10 파일, 2,536 LOC)
**현재 런타임:** Bun (single-binary compile, 5 cross-arch)
**목표 런타임:** Rust (cargo build, 5 cross-arch, native binary)
**상태:** Draft — review 전

---

## 0. TL;DR

axhub-helpers 를 Rust 로 완전 포팅하면 binary 크기 5~10배 감소 (50~90MB → 5~15MB), cold start 5배 향상 (~50ms → ~10ms), 메모리 baseline 80% 감소 (~30MB → ~5MB) 를 얻는다. 비용은 풀타임 8~12주 (1인 기준) 와 test fixture 재구축. 점진적 (모듈별 stub-and-replace) 전략으로 진행해 매 단계마다 release 가능 상태 유지.

**핵심 의사결정 (review 필요):**
1. **점진 vs 빅뱅** — 점진 (모듈별 교체) 권장. 4개월 동안 dual-runtime 코드베이스 운영.
2. **Keychain 전략** — `keyring` crate 단일화 (현재 mac/win/linux 따로) 권장.
3. **JWT 전략** — `jsonwebtoken` + `rustls` 권장. `josekit` 도 옵션.
4. **Test parity** — 498+ test baseline 유지 위해 TS test 를 그대로 두고 Rust 모듈은 별도 `cargo test` + `tests/integration/` 신설.
5. **Release pipeline** — cosign 서명은 그대로. cross-compile 은 `cross` 또는 GitHub Actions matrix.

---

## 1. 현재 상태 분석

### 1.1 모듈 인벤토리

| 파일 | LOC | 역할 | 외부 의존 | 복잡도 |
|------|-----|------|-----------|--------|
| `index.ts` | 509 | CLI entry, command dispatch, session-start, env shimming, Bun.stdin.text | Bun.spawnSync, fs | 높음 |
| `consent.ts` | 458 | Consent JWT mint+verify, parser hardening, token file (mode 0600 + O_NOFOLLOW), HMAC key lifecycle | jose (HS256 only), crypto.randomBytes/randomUUID, fs/promises (constants.O_NOFOLLOW) | **매우 높음** (parser+filesystem, mTLS 아님) |
| `list-deployments.ts` | 339 | Hub API client + **TLS SPKI pinning + X509 certificate verify** + deployment listing | fetch, **node:tls.connect**, **node:crypto.X509Certificate + createHash**, redact | **매우 높음** (TLS pin) |
| `resolve.ts` | 296 | App config resolve, identity, profile lookup | fs, env, fetch | 중 |
| `preflight.ts` | 257 | CLI version check, hub compat, semver | semver, fetch | 중 |
| `keychain-windows.ts` | 222 | Windows Credential Manager wrapper | child_process (powershell) | 중 |
| `catalog.ts` | 188 | NL command classifier (codegen target) | none | 낮음 |
| `keychain.ts` | 132 | macOS Keychain + Linux Secret Service wrapper | child_process (security/secret-tool) | 중 |
| `telemetry.ts` | 87 | Meta envelope emitter (jsonl append) | fs/promises | 낮음 |
| `redact.ts` | 48 | PII redaction utility | none | 낮음 |

**Σ 2,536 LOC.** 핵심 위험 모듈은 `list-deployments.ts` (TLS SPKI pinning + X509), `consent.ts` (JWT HMAC + parser + token file 보안), `keychain-windows.ts` (한국 EDR 호환).

**중요 정정:** 이전 작성된 plan 본문은 consent.ts 가 mTLS+X509 사용한다고 추정했으나 실제 source 검증 결과 잘못. consent.ts 는 jose 의 HS256 JWT mint/verify 만 사용 (대칭 HMAC). TLS+X509 는 list-deployments.ts 의 hub-api 직접 호출 fallback path 에 있음. 위험 profile 이 Phase 2 (list-deployments) 와 Phase 3 (consent + keychain) 양쪽 모두 매우 높음.

### 1.2 외부 의존 (npm)

| npm | Rust 매핑 | 비고 |
|-----|-----------|------|
| `jose` ^5.9.6 (consent.ts 만 사용, **HS256 only**) | `jsonwebtoken` 9.x | JWS-only 확인됨 (grep 결과 SignJWT+jwtVerify 만). josekit 불필요 |
| `semver` ^7.6.3 | `semver` 1.x crate + `regex` | preflight.ts:73 의 prerelease/build 의도적 drop **bug-for-bug parity** 필요. `semver::Version::parse` 로는 부족 |
| `zod` ^3.23.8 | `serde` + `validator` | parse 가 아닌 schema 검증은 직접 구현 또는 `garde` |
| `node:crypto.X509Certificate` (**list-deployments.ts 만**) | `x509-parser` + `sha2` | SPKI public key DER hash. consent.rs 와 무관 |
| `node:tls.connect` (**list-deployments.ts 만**) | `rustls` + `tokio-rustls` | hub-api leaf SPKI pinning. AXHUB_ALLOW_PROXY=1 옵트아웃 보존 필수 |
| `node:crypto.randomBytes/randomUUID` (consent.ts) | `getrandom` crate (직접) | **`rand::thread_rng` 금지** — HMAC key 는 OS CSPRNG 강제 |
| `node:fs constants.O_NOFOLLOW` (consent.ts) | `nix::fcntl::OFlag::O_NOFOLLOW` (Unix) + Windows 별도 ACL | symlink-injection 방어 보존 |
| `node:fs/promises` | `tokio::fs` | async 그대로 |
| `node:child_process.spawnSync` | `std::process::Command` | sync 1:1 |
| `node:os.homedir/tmpdir` | `dirs` crate | cross-platform |
| `fetch` (Bun built-in) | `reqwest` (rustls-tls feature) | OpenSSL 의존 회피 |
| `Bun.build --compile` | `cargo build --release` + `strip` | 단일 binary |

### 1.3 빌드/릴리스 파이프라인 (변경 영향)

- `package.json` scripts (`build:darwin-arm64` 등 5개) → `Cargo.toml` + GitHub Actions matrix
- `release.yml` → cosign 서명 단계는 유지, `bun build --compile` 부분만 `cargo build --release --target=...` 로 교체
- `windows-smoke.yml` / `sign-windows.yml.template` → 그대로 유지 가능 (서명 대상 binary 만 교체)
- `claude-cli-e2e.yml` → t1/t2/nightly matrix 는 그대로. `bun install` + `bun run build` 부분이 `cargo build` 로 변경
- Codegen 스크립트 (`codegen:catalog`, `codegen:version`) → Rust 의 `build.rs` 로 이전 또는 외부 스크립트 유지

---

## 2. 포팅 전략 — 점진 vs 빅뱅

### 2.1 권장: 점진 교체 (Strangler Fig 패턴)

**개념:** Rust 로 신규 binary `axhub-helpers-rs` 작성. CLI dispatcher 가 Rust 로 호출 가능한 명령은 Rust binary 로 위임, 미포팅 명령은 기존 TS binary 로 fallback.

**4단계 (각 2~3주):**

1. **Foundation (Week 1~3)** — Rust 프로젝트 scaffold, CI 추가, 가장 단순한 모듈부터 (`redact`, `telemetry`, `catalog`)
2. **Stateless 모듈 (Week 4~6)** — `preflight`, `resolve`, `list-deployments` 포팅. fetch + semver 기반.
3. **Security 모듈 (Week 7~9)** — `consent` (JWT + mTLS), `keychain` (mac/win/linux). 가장 위험.
4. **Entry 통합 (Week 10~12)** — `index.ts` 의 CLI dispatch 를 Rust 로. TS binary 제거, single binary `axhub-helpers` 로 복귀.

**장점:**
- 매 단계 release 가능. 회귀 발견 시 즉시 rollback.
- E2E test (claude-cli matrix) 가 매 PR 마다 검증.
- 팀 학습 곡선 흡수.

**단점:**
- 4개월간 dual-runtime 코드베이스 (Rust + TS 공존).
- IPC overhead — Rust 가 TS subprocess spawn 시 비용. 하지만 대부분 명령은 단일 호출이라 무시 가능.
- 빌드 산출물 2개 동시 관리.

### 2.2 대안: 빅뱅 (전체 재작성)

**개념:** feature freeze 4주, 한번에 전체 포팅, parity 달성 후 단일 PR merge.

**비용:** 풀타임 6~8주 + 회귀 위험 매우 높음. 권장하지 않음. 작은 helper 라 가능은 하나, 498 개 test parity + 5 platform 검증을 한번에 하는 부담이 크다.

---

## 3. 모듈별 포팅 상세

### 3.1 Phase 1: Foundation (Week 1~3)

**산출물:**
- `axhub-helpers-rs/` 디렉터리 (Cargo workspace 또는 별도 crate)
- `Cargo.toml` 의존성 lock
- GitHub Actions Rust job 추가 (build + test, fail 시 PR block)
- 첫 번째 포팅 모듈 3개

**모듈:**

#### `redact.rs` (48 LOC TS → ~80 LOC Rust 추정)
- 정규식 기반 PII 마스킹
- 의존: `regex` crate
- Test: TS 의 `redact.test.ts` 케이스를 그대로 Rust `#[test]` 로 이전
- 위험도: **낮음**

#### `catalog.rs` (188 LOC TS → ~250 LOC Rust)
- NL 분류기. codegen 대상이라 input 은 JSON corpus
- 의존: `serde_json`, `regex`
- 주의: `codegen:catalog` 스크립트가 TS 산출물을 만들고 있음 → Rust 의 `build.rs` 로 이전하거나 codegen 스크립트가 Rust source 도 생성하도록 dual-emit
- Test: `corpus.jsonl` (24KB+) 를 Rust 에서도 로드해서 동일 분류 결과 검증
- 위험도: **낮음**

#### `telemetry.rs` (87 LOC TS → ~150 LOC Rust)
- `~/.axhub/telemetry/*.jsonl` 에 envelope append
- 의존: `tokio::fs`, `serde_json`, `chrono`
- Test: temp dir 에 envelope 작성 후 readback 비교
- 위험도: **낮음**

### 3.2 Phase 2: Stateless 모듈 (Week 4~6)

#### `preflight.rs` (257 LOC TS → ~400 LOC Rust)
- CLI 버전 체크, hub 호환성, semver compare
- 의존: `semver`, `reqwest`, `tokio`
- 주의: `Could not resolve: 'semver'` 회귀 (CHANGELOG 22.x) 와 동일 시나리오 재현 → CI 에서 강제 검증
- Test: mock-hub fixture (`tests/fixtures/`) 재사용. Rust 측은 `httpmock` crate
- 위험도: **중**

#### `resolve.rs` (296 LOC TS → ~450 LOC Rust)
- 앱 식별자 resolve, profile lookup, identity merge
- 의존: `serde`, `reqwest`, `dirs`
- 주의: 환경변수 fallback 우선순위 + config 파일 병합 로직 정밀 검증
- Test: `tests/fixtures/profiles/` 케이스 그대로 사용
- 위험도: **중**

#### `list-deployments.rs` (339 LOC TS → ~500 LOC Rust)
- Hub API 클라이언트, deployment 리스트, redact 적용
- 의존: `reqwest`, `serde`, `tokio-stream`
- Test: `list-deployments.test.ts` 의 19개 케이스 → Rust integration test
- 위험도: **중**

### 3.3 Phase 3: Security 모듈 (Week 7~9) — 최고 위험

#### `consent.rs` (458 LOC TS → ~700 LOC Rust)
- OAuth consent, JWT issue + verify, mTLS, nonce, X509 pinning
- 의존: `jsonwebtoken` + `rustls` + `tokio-rustls` + `x509-parser` + `webpki`
- **위험 포인트:**
  - X509 fingerprint pinning 로직 (`createHash("sha256").update(cert.raw)`) — Rust 에서 동일 byte 비교 필요
  - JWT clock skew tolerance — jose 의 `clockTolerance` 옵션 매핑
  - mTLS 클라이언트 인증서 로드 — Rust 의 `rustls::ClientConfig` 빌더 패턴
  - JWE (encrypted JWT) 가 사용 중이면 `josekit` 추가 필요. JWS 만이면 `jsonwebtoken` 충분
- Test: `consent.test.ts` 19KB (가장 큰 테스트) — 모든 케이스 1:1 이전
- 위험도: **매우 높음** — 보안 회귀 시 token 도용 가능. security-reviewer agent 필수.

#### `keychain.rs` (132 LOC TS → ~200 LOC Rust)
+ `keychain-windows.rs` (222 LOC TS → ~250 LOC Rust)
- macOS Keychain (security CLI) + Linux Secret Service + Windows Credential Manager
- **권장:** `keyring` crate 단일 추상화 — 3 platform 통합
- 의존: `keyring` 3.x crate
- 주의: `keyring` 은 OS 네이티브 API 사용 (security CLI subprocess 안 함). 더 빠르고 견고하지만 동작 차이 검증 필요
- Test: `keychain.test.ts` + `keychain-windows.test.ts` 케이스 이전. 단, `keyring` 으로 통합하면 일부 OS-specific 케이스 (security CLI flag 등) 는 사라짐
- 위험도: **높음** — 토큰 저장소 손상 시 사용자 재로그인 강제
- **고려사항:** Linux Secret Service 가 headless 환경 (CI, Docker) 에서 unavailable 시 fallback. 현재 TS 구현 동작과 동일하게.

### 3.4 Phase 4: Entry + 통합 (Week 10~12)

#### `main.rs` (index.ts 509 LOC → ~800 LOC Rust)
- CLI dispatcher (`clap` derive)
- session-start hook, env shim, command routing
- 의존: `clap` 4.x, `anyhow`, `tracing`
- **위험 포인트:**
  - `spawnSync` 패턴이 많음 (claude-cli, security CLI 등) → `std::process::Command` 1:1 매핑. exit code + stdout/stderr 인코딩 정밀 검증
  - Bun 의 `Bun.argv` slicing 차이 — clap 의 `args_from_argv` 로 해결
  - error propagation — TS 의 throw → Rust 의 `Result<T, anyhow::Error>` 로 전환. 사용자에게 노출되는 에러 메시지 한글 워딩 보존
- Test: `axhub-helpers.test.ts` 11.5KB + e2e 매트릭스 전체
- 위험도: **높음**

#### TS 모듈 제거
- `bun build` script 제거
- `package.json` 의 `engines.bun` 제거 (또는 plugin runtime 만 Bun 의존 유지)
- README + install scripts 의 Bun 참조 제거

---

## 4. 빌드/릴리스 파이프라인

### 4.1 Cargo workspace 구조

```
axhub/
├── src/axhub-helpers/        # 기존 TS (Phase 4 까지 유지)
├── crates/
│   ├── axhub-helpers/        # main binary
│   │   ├── src/main.rs
│   │   ├── src/consent.rs
│   │   ├── src/keychain.rs
│   │   └── ...
│   └── axhub-codegen/        # build.rs helpers (catalog, version)
│       └── src/lib.rs
├── Cargo.toml                # workspace
└── tests/                    # 기존 TS 테스트 + 신설 Rust integration
    └── integration/
        └── consent_e2e.rs
```

### 4.2 Cross-compile

**[수정] Windows 는 MSVC 강제 (Authenticode + EDR reputation).**

```bash
# Linux/macOS: cross 도구 가능
cross build --release --target aarch64-apple-darwin
cross build --release --target x86_64-apple-darwin
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu

# Windows: MSVC 강제 — cross 로 GNU 가능하지만
# sign-windows.yml.template:49 가 PE32+ MSVC 기반 signtool 가정
# → GH Actions windows-latest runner 에서 native 빌드 필수
cargo build --release --target x86_64-pc-windows-msvc
```

**근거 (Eng phase 발견):**
- Authenticode signtool 은 MSVC PE32+ 산물에 최적화. GNU 산물은 SmartScreen reputation 별도
- 한국 EDR (V3, AhnLab) 가 GNU 와 MSVC 산물 다르게 인식 가능
- 추가 비용: Windows native runner 매트릭스 (+1주 CI 설정)

**대안 (덜 권장):** GitHub Actions matrix — 모든 OS 별로 native 빌드. macOS notarization 시 Apple silicon runner 필수. cross 의 cache 이점 포기.

### 4.3 release.yml 변경

| 단계 | 현재 | 변경 후 |
|------|------|---------|
| Build | `bun run build:all` (5 binary) | `cargo build --release` matrix |
| Strip | (Bun 자동) | `strip` + `cargo-strip` |
| Sign | cosign sign | cosign sign (변경 없음) |
| Upload | gh release upload | gh release upload (변경 없음) |
| Verify | smoke test | smoke test + version assert (release-check.ts → release-check.rs) |

### 4.4 codegen

- `codegen:version` — install.sh / install.ps1 / index.ts 동기화 → Rust 로 이전 시 `src/version.rs` 자동 생성하는 `build.rs` 로 흡수
- `codegen:catalog` — corpus.jsonl 에서 분류 lookup 테이블 생성 → 동일하게 `build.rs` 흡수

---

## 5. 테스트 전략

### 5.1 현재 baseline (Phase 18)

- `bun test` 498+ pass
- `tsc --noEmit` clean
- E2E matrix t1/t2/nightly (claude-cli)
- skill:doctor / lint:tone / lint:keywords clean

### 5.2 포팅 중 정책

- **TS test 는 살아 있는 한 모두 통과해야 함.** 모듈을 Rust 로 옮긴 시점에 해당 TS test 는 deprecation comment 후 삭제. Rust 측에 동등 test 가 먼저 작성되어야 함 (TDD).
- 각 Rust 모듈은 `cargo test` 단위 테스트 + `tests/integration/` 통합 테스트.
- E2E matrix 는 양쪽 binary 를 모두 검증 (Phase 4 까지). Rust binary 는 새 환경변수 `AXHUB_HELPERS_RUNTIME=rust` 로 분기.
- mock-hub fixture (`tests/fixtures/`) 는 TS / Rust 양쪽에서 재사용. JSON 기반이라 호환.

### 5.3 회귀 방지

- `consent.rs` 만 별도 fuzz target 작성 (`cargo-fuzz`) — JWT + X509 파싱 회귀 검출
- semver 회귀 (CHANGELOG 22.x 전례) 재발 방지: CI 에 `cargo build --release --target=... && ./binary version` 강제

---

## 6. 위험 요소 + 완화

| 위험 | 영향 | 가능성 | 완화 |
|------|------|--------|------|
| `consent.rs` 보안 회귀 (JWT 검증 우회) | 매우 높음 | 중 | security-reviewer agent + cargo-audit + 외부 페네스트 |
| Bun-specific API 누락 (e.g., `Bun.serve`) | 중 | 낮음 | grep `Bun\.` survey 결과 0 건 가정. 발견 시 `tokio` / `hyper` 매핑 |
| `keyring` crate 동작 차이 (특히 Linux Secret Service) | 높음 | 중 | headless fallback 명시. Docker / WSL / 헤드리스 CI 에서 별도 검증 |
| Cross-compile 실패 (특히 Windows + GNU) | 중 | 중 | GitHub Actions native matrix 로 fallback |
| 한글 에러 메시지 워딩 drift (해요체 보존) | 낮음 | 높음 | 메시지 catalog 분리 (`messages.rs`) + lint:tone 동등 검사 추가 |
| 팀 Rust 경험 부족 → maintenance burden | 중 | 중 | Phase 1~2 동안 페어 프로그래밍 + 코드 리뷰 강화 |
| Plugin runtime 의존성 (Claude Code plugin) | 중 | 낮음 | helper binary 만 Rust 화. plugin manifest / SKILL.md 는 그대로 |

---

## 7. 단계별 Exit Criteria

각 Phase 끝에 다음 모두 만족해야 다음 Phase 진행:

**Phase 1:**
- [ ] Cargo workspace + CI green
- [ ] 3 모듈 (redact/catalog/telemetry) Rust 포팅 완료
- [ ] 해당 TS test 동등성 검증
- [ ] release.yml 에 Rust build 단계 추가 (실 binary 는 unused, 검증만)

**Phase 2:**
- [ ] preflight/resolve/list-deployments Rust 포팅
- [ ] mock-hub fixture 양쪽에서 동일 결과
- [ ] CI 에 Rust binary smoke test 추가

**Phase 3:**
- [ ] consent/keychain Rust 포팅
- [ ] security-reviewer agent 통과
- [ ] keyring 의 headless fallback 검증
- [ ] cargo-fuzz 24h run 무결함

**Phase 4:**
- [ ] main.rs CLI dispatcher 완성
- [ ] TS binary 제거 PR
- [ ] E2E matrix 100% green (TS binary 없이)
- [ ] release v1.0.0-rust 태깅
- [ ] CHANGELOG 명시 + 마이그레이션 가이드

---

## 8. Rollback 전략

각 Phase 별 release tag 보존. 회귀 발견 시:
1. `git revert <merge-commit>` + 즉시 patch release
2. 사용자에게 `axhub update --force-version <prev>` 안내
3. cosign 서명된 이전 binary 는 GH release 에서 그대로 download 가능

Phase 4 (TS 제거) 직후 1주간 monitor. 회귀 없으면 다음 minor 에 TS 코드 완전 삭제.

---

## 9. 비-목표 (NOT in scope)

- Plugin manifest (`plugin.json`, `marketplace.json`) Rust 화 — 이건 Claude Code plugin 메타이라 helper 와 무관
- SKILL.md / NL trigger 어구 변경 — frontmatter `description:` baseline 유지
- 신규 기능 추가 — 순수 포팅. 새 명령어 / 새 옵션 금지. 비교 가능성을 위해.
- Bun runtime 의존성을 **plugin** 측에서도 제거 — 본 plan 은 helper 만 다룸

---

## 10. 의사결정 필요 항목 (Final Gate)

리뷰 시 답해야 할 질문:

1. **Q1:** 점진 (4개월) vs 빅뱅 (8주 freeze) — 어느 쪽?
   **권장:** 점진. dual-runtime 비용 < 회귀 위험.

2. **Q2:** Keychain 단일 추상화 (`keyring` crate) vs 현재 분리 구조 유지?
   **권장:** `keyring`. 3 platform 코드 -50%, 유지보수 단순.

3. **Q3:** JWT 라이브러리 — `jsonwebtoken` (JWS-only) vs `josekit` (JWS+JWE)?
   **답:** `consent.ts` 의 JWE 사용 여부 grep 후 결정. 없으면 `jsonwebtoken` 단독.

4. **Q4:** Test parity 방식 — TS test 그대로 두고 점진 삭제 vs 한번에 Rust 로 이전?
   **권장:** 점진. 모듈 포팅 완료 시점에 해당 TS test 제거.

5. **Q5:** Plugin runtime 의 Bun 의존성도 같이 제거? 또는 helper 만 Rust 화?
   **권장:** helper 만. plugin 은 별도 plan 으로.

6. **Q6:** 1인 풀타임 8~12주 투자 회수 가능한가? (binary size 불만, cold start 통증, 보안 요구 중 어느 것이 driver?)
   **답:** 통증 driver 명시 필요. 통증 약하면 plan 보류 권장.

---

## 11. Next Step

이 plan 승인 시:
1. Q1~Q6 결정 → ADR 1건 (`.omc/adr/0001-rust-port-decision.md`)
2. Phase 1 issue 생성 (GitHub issue 3개: redact / catalog / telemetry)
3. Cargo workspace scaffold PR 작성
4. CI Rust matrix 추가 PR 작성 (block 안 함, 정보용)

승인 거부 시:
- 통증 명시 (어떤 user feedback / metric 이 driver 인지) 후 재검토
- 또는 hot path 만 Rust addon (예: consent.rs 만 Rust ffi, 나머지 TS 유지) 으로 scope 축소

---

**작성자:** Claude (Opus 4.7)
**Review 필요:** security-reviewer (consent), code-reviewer (전체), 사용자 (Q1~Q6)

---

# /autoplan Review (시작: 2026-04-29T11:30Z)

## Phase 1: CEO Review

### Step 0A — Premise Challenge (직접 분석)

**Plan 에 implicit premise 5개:**

| # | Premise | 검증 | 위험 |
|---|---------|------|------|
| P1 | "TS→Rust 포팅이 의미 있는 ROI 를 가진다" | **검증 안 됨**. Plan §1.0 자체가 "Q6: 통증 driver 명시 필요. 통증 약하면 plan 보류 권장" 라고 적혀 있음. driver 가 사용자 피드백인지, internal metric 인지 불명. | 매우 높음 — 통증 없으면 8~12주 노력 손실 |
| P2 | "Bun runtime overhead 가 실측 문제다" | **검증 안 됨**. Bun cold start 50ms, memory 30MB 는 generic estimate. axhub-helpers 가 매번 호출되는 hot path 인지, 사용 빈도 데이터 없음. | 높음 — 이론적 이득이지만 실제 사용자 경험 영향 미미할 수 있음 |
| P3 | "Binary size 50~90MB 가 사용자 마찰이다" | **부분 검증**. CHANGELOG 에 axhub update 다운로드 관련 사용자 불만 기록 없음. 하지만 cosign 서명된 5 binary 배포라 사이즈는 명백한 비용 |
| P4 | "Rust 학습 곡선 + maintenance burden 이 흡수 가능하다" | **검증 안 됨**. Plan §6 "팀 Rust 경험 부족 → maintenance burden ↑" 완화책이 "페어 프로그래밍" 으로 약함. 1인 팀이면 bus factor 1 |
| P5 | "consent.ts 의 mTLS+JWT+X509 정확한 1:1 포팅 가능하다" | **부분 검증**. jose 의 일부 옵션 (clockTolerance 등) 매핑은 있으나 JWE 사용 여부 미확인. |

**0A 판정:** P1, P2, P4 가 핵심 위험. 사용자 confirmation 필수 (premise gate).

### Step 0B — Existing Code Leverage Map

| Sub-problem | 기존 자산 | 재사용 가능성 |
|-------------|-----------|---------------|
| Cross-arch binary 배포 | release.yml + cosign 서명 인프라 | 높음 — pipeline 만 바꾸면 됨 |
| Test fixture (mock-hub, claude-cli matrix) | tests/fixtures/, tests/e2e/ | 높음 — JSON 기반이라 Rust 도 사용 가능 |
| 한글 메시지 catalog | nl-lexicon baseline (lint:keywords) | 중 — Rust 측 messages.rs 에 동등 lock 필요 |
| Codegen scripts | scripts/codegen-*.ts | 중 — Rust build.rs 로 흡수 가능하지만 dual-emit 기간 필요 |
| Plugin manifest | plugin.json, marketplace.json, SKILL.md | 영향 없음 — helper 만 포팅 |

### Step 0C — Dream State Diagram

```
CURRENT (v0.1.23):
  Bun TS helper (50~90MB binary, ~50ms cold start, ~30MB mem baseline)
  └── 5 cross-arch binaries via bun build --compile
      └── cosign 서명 + GH release

THIS PLAN (Rust port complete):
  Rust native binary (5~15MB, ~10ms cold start, ~5MB mem)
  └── cargo build --release matrix
      └── cosign 서명 + GH release (변경 없음)

12-MONTH IDEAL:
  axhub CLI 가 사용자 마찰 zero (install < 5초, cold start invisible)
  + plugin runtime 도 native (Bun 의존 완전 제거)
  + 보안 감사 통과 (memory-safe, reproducible build)
```

**Delta:** 현재 plan 은 12-month ideal 의 helper 부분만 다룸. plugin runtime 은 별도 work. Plan §9 NOT in scope 에 명시되어 있음 — 합리적.

### Step 0C-bis — Implementation Alternatives Table

| 접근 | Effort | Risk | Pros | Cons |
|------|--------|------|------|------|
| **A. 점진 (Strangler Fig)** | 8~12주 | 중 | 매 phase release 가능, 회귀 시 빠른 rollback, 학습 흡수 | dual-runtime 4개월, IPC overhead 가능 |
| **B. 빅뱅** | 6~8주 freeze | 매우 높음 | 단일 PR, dual-runtime 부담 없음 | 회귀 시 영향 거대, 4개월간 user-facing release 정지 |
| **C. Hot-path-only Rust addon** | 3~4주 | 낮음 | 최소 노력으로 핵심 이득 (consent.ts FFI), TS 유지 | 두 언어 영구 공존, FFI 복잡성 영구화 |
| **D. 보류 + Bun 최적화** | 1~2주 | 매우 낮음 | 즉시 release, Rust 학습 비용 없음 | 이론적 ceiling 낮음, binary size 개선 한계 |

**Plan §2.1 권장: A (점진).** 하지만 **C (hot-path-only)** 가 §1.0 "현재 통증 약하면 회수 안 됨" 시나리오에서 더 합리적. **D (보류)** 는 통증 명시 없을 때 최선.

### Step 0D — Mode Selection

**Mode: SELECTIVE EXPANSION** — Plan 자체는 이미 §10 의사결정 항목 6개 명시. 핵심은 premise (Q6 driver) 미해결. 새 scope 추가 없이 premise 검증과 위험 보강.

### Step 0E — Temporal Interrogation

| 시점 | 무슨 일 일어나는가 |
|------|---------------------|
| HOUR 1 | Q1~Q6 결정 + ADR 작성 |
| WEEK 1 | Cargo workspace scaffold PR + CI Rust matrix 추가 |
| WEEK 3 | 첫 3 모듈 (redact/catalog/telemetry) Rust 포팅 + TS test 동등성 검증 |
| WEEK 6 | preflight/resolve/list-deployments 포팅 + mock-hub fixture 양쪽 검증 |
| WEEK 9 | consent + keychain 포팅 — security-reviewer 통과 (최고 위험 구간) |
| WEEK 12 | main.rs 통합 + TS binary 제거 → v1.0.0-rust 태깅 |
| WEEK 13~16 | Monitor 기간. 회귀 발견 시 hotfix release |
| MONTH 6 | TS 코드 완전 삭제 + plan §9 의 "plugin runtime 의 Bun 제거" 별도 plan 시작 가능 |

**위험 핫스폿:** WEEK 7~9 (consent 포팅). 보안 회귀 시 사용자 token 도용 가능.

### Step 0F — Mode Confirmation

SELECTIVE EXPANSION 유지. 추가 scope 0개, 보강 항목 (premise 검증) 만.

### Step 0.5 — Dual Voices

#### CLAUDE SUBAGENT (CEO — strategic independence)

**Verdict: STOP. Do not approve.**

7 critical findings:
1. **[CRITICAL]** Plan validates itself, then asks if problem exists. Q6 buried at end. Should be Q1, "no" should kill plan in 30 min not 12 weeks
2. **[CRITICAL]** Headline benefits unverified napkin math. axhub-helpers is network-bound IO (mTLS+Hub round-trip). 50ms→10ms cold start invisible against 800ms network. Run hyperfine first
3. **[HIGH]** Cheaper alternatives dismissed in one line. `bun build --minify` + UPX + lazy-load consent + remove unused JWE = often 30~70% size cut without rewrite
4. **[HIGH]** Bus factor 1, Rust experience unstated. Solo maintainer (`realitsyourman`). consent.rs requires expert rustls+JWT+X509+clock-skew. One miss = silent token-validation bypass
5. **[HIGH]** Opportunity cost unpriced. 1Q feature velocity. Active investment in Phase 22.x e2e+skills. Plan lists ZERO deferred features
6. **[HIGH]** 6-month regret memo plausible: "Binary 90→12MB, 3 users mentioned. Competitor shipped backlog feature. consent.rs CVE on missed clock-skew"
7. **[MEDIUM]** Strangler Fig framing hides dual-runtime tax. Strangler only makes sense for multi-team systems, not 2,536 LOC

**Recommendation:** Reject. Approve 1-week premise-validation sprint instead — hyperfine top-5, Bun-optimized delta, user signal audit.

#### CODEX SAYS (CEO — strategy challenge)

**Founder-level recommendation: pause, do not approve.**

8 strategic blind spots:
1. ROI 미해결인데 "final gate" 가 아닌 "first gate" 여야 함
2. Engineering aesthetics 를 market leverage 보다 우선. 활성화/리텐션/지원부담/엔터프라이즈/업데이트 안정성 metric 미명시
3. "Rust port" 가 problem validation 전에 solution 으로 framed. Install friction → installer/CDN. Security trust → audit/SBOM. Latency → profile first
4. 경쟁 위험 = opportunity cost. 6개월 후 "2,536 LOC 다시 썼고 사용자는 신경 안 씀"
5. Maintenance drag underpriced. Solo 면 페어링 mitigation 은 fictional
6. Security argument inverted. consent 재작성 = 최고 신뢰 surface 에 fresh auth regression
7. Gradual = release risk 감소 but strategic risk 증가. 4개월 dual-runtime 으로 unproven payoff carry
8. Alternatives 너무 빨리 dismissed: measure first, Bun packaging, lazy-load, hot-path-only port, update UX, do nothing

**Recommendation:** 1주 validation sprint — pain driver 정의, real telemetry/support 증거 baseline, kill threshold, cheapest mitigation 테스트.

#### CEO DUAL VOICES — CONSENSUS TABLE

```
═══════════════════════════════════════════════════════════════
  Dimension                              Claude  Codex  Consensus
  ──────────────────────────────────────  ──────  ─────  ─────────
  1. Premises valid?                       NO      NO    DISAGREE-w-plan
  2. Right problem to solve?               NO      NO    DISAGREE-w-plan
  3. Scope calibration correct?            NO      NO    DISAGREE-w-plan
  4. Alternatives sufficiently explored?   NO      NO    DISAGREE-w-plan
  5. Competitive/market risks covered?     NO      NO    DISAGREE-w-plan
  6. 6-month trajectory sound?             NO      NO    DISAGREE-w-plan
═══════════════════════════════════════════════════════════════
```

**6/6 confirmed disagreement with plan direction.** This is the strongest possible USER CHALLENGE signal — both models independently arrived at "pause + 1-week validation sprint" as the correct first step.

#### Convergent Recommendations (양 모델 합의)

1. **Pre-port: 1주 validation sprint** with 3 deliverables:
   - `hyperfine` benchmark of top-5 commands (current binary) — 실측 cold start
   - Bun-optimized binary delta (`--minify` + UPX + lazy-load consent + JWE grep) — Rust 없이 얼마나 줄어드나
   - User signal audit (issues, support tickets, install telemetry) — 실제 통증 driver
2. **Kill threshold:** validation 결과 (1) p50 latency 개선 < 100ms AND (2) Bun 최적화로 size 50% 이상 절감 가능 AND (3) user 통증 신호 약함 → plan 폐기
3. **Partial scope option:** kill threshold 통과해도 consent.rs 만 hot-path Rust addon 또는 완전 보류 가능. all-or-nothing 거짓 dichotomy

### Step 1~10 — Review Sections (보강 분석)

Plan 의 §1~§11 가 이미 mode 분석 + alternative 비교 + risk table + exit criteria + rollback 모두 다룸. CEO dual voices 가 발견한 핵심 갭 (premise 검증) 외 추가 critical issue 없음. 다만 다음 보강 필요:

**§1 Architecture (no new finding)** — 모듈 인벤토리 정확. consent.ts 의 JWE 사용 여부 grep 미수행 — Phase 1 시작 전 prerequisite.

**§5 Test Strategy (gap)** — 498 test parity 가 numerical 지표만 있고 fixture 의존성 graph 없음. mock-hub fixture 가 TS-specific 한 부분 (예: bun:test 의 mock 헬퍼) 있으면 Rust 측 재구축 비용 미계상.

**§6 Risks (gap)** — bus factor 1 누락. solo maintainer = consent.rs 보안 회귀 시 hotfix 가능 시간 제한.

**§7 Exit Criteria (no new finding)** — Phase 별 measurable. OK.

### "NOT in scope" (CEO mode 결정)

- 신규 기능 추가 (deploy/recover/update 명령어 변경 등)
- Plugin runtime Bun 제거
- SKILL.md / nl-lexicon trigger 변경

### "What already exists" (재사용)

- release.yml + cosign 서명 인프라 (변경 최소)
- tests/fixtures/mock-hub (JSON 기반, Rust 호환)
- nl-lexicon baseline lock (한글 메시지)

### Error & Rescue Registry

| 시나리오 | TS 동작 | Rust 동작 (포팅 후) | 회귀 위험 |
|----------|---------|---------------------|-----------|
| consent JWT clock skew | jose `clockTolerance: 30s` | jsonwebtoken `leeway(30)` | **매우 높음** — 1초 차이로 토큰 거부 |
| keychain unavailable (headless Linux) | `secret-tool` 부재 시 fallback file | `keyring` crate 의 platform fallback 차이 | 높음 |
| mTLS handshake 실패 | TS error throw → 한글 에러 메시지 | Rust `Result::Err` → 메시지 catalog | 중 |
| semver compare edge | jose-style "1.0.0-rc.1" prerelease | `semver` crate 의 prerelease 비교 미세 차이 | 중 |
| Hub API 401 | TS retry once + reauth | Rust 동등 로직 직접 작성 | 중 |

### Failure Modes Registry

| 모드 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| consent.rs JWT 검증 우회 | 중 | **치명** (token 도용) | cargo-fuzz 24h + 외부 페네스트 |
| keyring crate 의 Linux Secret Service flaky | 중 | 높음 (재로그인 강제) | headless fallback 명시 + 컨테이너 테스트 |
| Cross-compile fail mid-release | 낮음 | 중 (release 지연) | GH Actions native runner fallback |
| 한글 메시지 워딩 drift | 높음 | 낮음 (UX) | messages.rs catalog + lint:tone 동등 |
| Codex JWE 사용 누락 | 낮음 | 매우 높음 (consent 동작 안 함) | Phase 1 시작 전 grep prerequisite |

### Dream State Delta

이 plan 은 12-month ideal 의 **30%** 만 다룸. helper Rust 화 = "binary size 친화 + cold start 절감". 12-month ideal 의 다른 70% (plugin runtime native, 보안 감사 통과, install < 5초) 는 별개 work. **이 plan 만으로 사용자 마찰 zero 안 됨.**

### Phase 1 Completion Summary

| 항목 | 상태 | 비고 |
|------|------|------|
| Premise challenge | ✓ | P1/P2/P4 검증 안 됨 → critical |
| Existing code map | ✓ | release infra + fixture 재사용 가능 |
| Dream state diagram | ✓ | 30% coverage only |
| Alternatives table | ✓ | Option D (Bun 최적화) underexplored |
| Mode-specific analysis | ✓ | SELECTIVE EXPANSION 유지 |
| Temporal interrogation | ✓ | WEEK 7~9 핫스폿 (consent) |
| Mode confirmation | ✓ | scope expansion 0개 |
| Dual voices | ✓ | 6/6 DISAGREE-w-plan |
| Error/Rescue registry | ✓ | 5건 |
| Failure modes registry | ✓ | 5건 |

**Phase 1 verdict:** Plan 은 internally consistent 하지만 **외부 premise (Q6 driver)** 가 미검증. dual voices 양쪽 모두 1주 validation sprint 권장. 이는 **USER CHALLENGE** — 사용자가 명시한 방향 ("Rust 완벽 포팅 plan 작성") 을 두 모델이 동의하여 거부.

> **Phase 1 complete.** Codex: 8 concerns. Claude subagent: 7 issues. Consensus: 0/6 confirmed (all 6 DISAGREE-with-plan). USER CHALLENGE surfaced.

### Premise Gate — User Decision

**User answer (2026-04-29):** "ts로 구성된거 전부 다 rust로 완벽 포팅할꺼야"

USER CHALLENGE **REJECTED by user**. 사용자가 명시적으로 두 모델의 합의 (1주 validation sprint 먼저) 거부하고 full Rust 포팅 진행 선택. 추가로 "전부 다, 완벽" 강조 = scope 축소 + partial port + hot-path-only 옵션 모두 배제.

User sovereignty 원칙: 사용자가 모델이 못 보는 context (제품 비전, 시장 타이밍, 개인 학습 목표, 보안 요구사항 등) 보유. 모델 합의는 recommendation 이지 decision 아님. **Plan 그대로 진행.**

**단, 두 모델의 우려를 risk 로 기록:**

| Risk (acknowledged but accepted) | 출처 | 사용자 결정 시 가정 |
|----------------------------------|------|---------------------|
| Premise (통증 driver) 미검증 | Codex+Subagent 합의 | 사용자가 internal context 로 판단 |
| 8~12주 opportunity cost (기능 velocity 손실) | Codex+Subagent 합의 | 사용자가 product priority 결정권 |
| Solo maintainer + bus factor 1 | Subagent | 사용자가 학습/유지보수 의지 보유 |
| consent.rs 보안 회귀 surface | Codex+Subagent 합의 | Phase 3 에서 security-reviewer 강제 |
| 6-month regret memo 가능성 | Subagent | rollback 전략 (§8) 으로 완화 |

**Mandatory mitigations (사용자 결정 후에도 강제):**
1. Phase 1 시작 전: `consent.ts` 의 JWE 사용 여부 grep — JWS-only 면 `jsonwebtoken`, JWE 사용 시 `josekit` (Q3 답)
2. Phase 1 시작 전: `Bun\.` API 사용 survey — 누락 시 `tokio`/`hyper` 매핑 계획
3. Phase 3 (consent/keychain): security-reviewer agent + cargo-audit + cargo-fuzz 24h **필수**
4. Phase 4 (TS 제거) 직후 1주 monitor 의무 — 회귀 발견 시 즉시 hotfix release

## Phase 3: Eng Review

### Step 0 — Scope Challenge (실제 source 검증)

**Subagent + Codex 양 모델이 plan 의 §1-3 source drift 발견.** Plan 작성자 (나) 가 grep 없이 추정으로 inventory 작성한 결과 critical inaccuracy.

#### 실제 source 검증 결과

| 검증 항목 | Plan 주장 | 실제 source | Verdict |
|----------|-----------|-------------|---------|
| consent.ts 외부 의존 | jose, tls, crypto (X509) | jose (HS256 HMAC only), randomBytes/randomUUID. **TLS/X509 없음** | **WRONG** |
| TLS + X509 SPKI pinning 위치 | consent.ts | **list-deployments.ts** (HUB_API_SPKI_SHA256_PINS, X509Certificate, tls.connect) | **WRONG** |
| JWT leeway | "jose clockTolerance 30s" | jose default = 0, code 도 옵션 미지정 = 0 leeway | **WRONG** (silent widen risk) |
| Bun-specific API | "0 건 가정" | 5건: keychain-windows.ts, index.ts (Bun.stdin.text), preflight.ts (Bun.spawnSync), keychain.ts (2건) | **WRONG** |
| preflight semver | "1:1 매핑" | regex 로 prerelease/build 의도적 제거 (preflight.ts:73). Rust 도 동일 contract 복제 필요 | **PARTIAL** |
| Test count | "498+ pass" | 실제 428 `test(` + 499 `test/describe` 선언 (codex grep) | **APPROX** |

#### 위험 profile 재배치

Plan 의 Phase 2/3 위험 ranking 잘못. 실제로:

| Module | 실제 위험 (재평가) | 원래 plan 위험 | 차이 |
|--------|------|---------|------|
| **list-deployments.ts** | **매우 높음** — TLS SPKI pinning + X509 + AXHUB_ALLOW_PROXY 옵트아웃 + corporate proxy 호환 | 중 | +2단계 |
| consent.ts | 높음 — HMAC key lifecycle (file mode 0600) + O_NOFOLLOW symlink guard + parseAxhubCommand 5-level recursion + binding 정확 비교 | 매우 높음 (X509 가정) | -1단계 (다른 위험 발견) |
| keychain-windows.ts | 매우 높음 — Korean EDR (V3/AhnLab) PowerShell+PInvoke, AMSI detection, 한글 fallback copy. `keyring` crate 로 단일화 시 EDR signature 불확실 | 중 | +2단계 |
| preflight.ts | 중 — regex prerelease drop bug-for-bug parity 필요 | 중 | 동일 |
| 그 외 (resolve, telemetry, redact, catalog) | 낮음 | 낮음 | 동일 |

### Step 0.5 — Dual Voices

#### CLAUDE SUBAGENT (Eng — 독립 검토)

10 critical+high+medium findings:
- **C1 [CRITICAL]** consent.ts crypto stack 잘못 — HMAC HS256 only, mTLS/X509 없음. Phase 3 dependencies (rustls, x509-parser, webpki) 전부 drop. `hmac + sha2 + jsonwebtoken` 충분
- **C2 [CRITICAL]** JWT leeway 0 잠금 필요 — port 가 leeway(30) 추가하면 silent auth window 확대
- **H1 [HIGH]** keyring crate ≠ go-keyring 인터롭. axhub-cli 가 `go-keyring-base64:` envelope 로 write, helper 가 read 만. Rust `keyring` crate 가 동일 attribute key 로 read 가능한지 spike 필수
- **H2 [HIGH]** Korean EDR (V3/AhnLab) Windows PowerShell PInvoke 의 EDR-friendly 동작 — Rust 직접 `windows-rs` 호출 시 different EDR signature 발생 가능. v0.1.5 코드사이닝 전 시기 사용자 마찰 재발 위험
- **H3 [HIGH]** Windows MSVC 강제 — Authenticode signing path 가 PE32+ MSVC 가정. plan §4.2 `x86_64-pc-windows-gnu` 잘못. MSVC 면 cross-compile 불가, native runner 필수 (+1주)
- **H4 [HIGH]** bun:test mock 은 port 안 됨. 30~50% test 가 단순 port 아닌 rewrite. 2~3주 추가 필요
- **M1 [MED]** spawn 의 exit code + signalCode 의미 다름. EDR detection 의 0xC0000409 status code Windows-specific
- **M2 [MED]** Windows codepage — 한글 메시지 CP949 corruption 위험 (chcp 65001 + UTF-8 OutputEncoding 명시 필요)
- **M3 [MED]** O_NOFOLLOW symlink guard — Rust 의 nix crate 사용 + Windows ACL 별도 처리
- **M4 [MED]** `getrandom` crate pin — `rand::thread_rng` 금지 (security-relevant HMAC key)

**Net effect:** Phase 3 위험 **과대**, Phase 4 (Windows MSVC + EDR + test rewrite) **과소**. 현실적 추정 **10~14주** (8~12주 아님).

#### CODEX SAYS (Eng — adversarial)

8 architectural findings (verdict: source-of-truth drift):
1. **§1.2 dependency mapping materially wrong** — consent.ts uses jose for HS256 only. `X509 → x509-parser` and `tls → rustls` 는 list-deployments.ts (REST fallback) 의 것
2. **Phase 3 risk profile mis-stated** — 실제 위험: shell-command parser hardening (consent.ts:242) + file-mode/O_NOFOLLOW (consent.ts:151) + HMAC key lifecycle + binding exact equality + session namespacing + expiry semantics
3. **JWT leeway claim wrong** — jose default 0, code 미지정 = 0. plan 의 "30s tolerance" 는 silent widen
4. **preflight semver under-scoped** — preflight.ts:73 의 regex 가 prerelease/build metadata 의도적 drop. Rust `semver::Version::parse` 로는 동일 contract 안 됨. bug-for-bug parity 필요
5. **Test parity 많이 어려움** — 428 test() + 499 test/describe. Bun-specific compiled-binary contract + plugin packaging + Korean wording assertions 포함. 단순 "Rust tests + fixtures" 매핑 불가
6. **Windows GNU 선택 under-analyzed** — Authenticode signtool.exe + sign-windows.yml.template 가 PE32+ MSVC 가정. EDR 위험 동기화 필요
7. **Korean EDR/V3/AhnLab 은 optional migration detail 아님** — keychain-windows.ts:117 의 EDR/AMSI detection 은 product behavior. plan 이 OS-specific test 를 removable 로 본 건 위험
8. **Release pipeline 가정 shallow** — 현재 self-hosted Linux ARM64 runner 가 Bun compile targets 5개. Rust 면 real matrix + cache strategy + target linker + 별도 Windows signing artifact flow 필요. "cargo build + strip" 은 drop-in 아님

**Required plan correction:** §1.1-§3 를 실제 module 기준으로 재작성. consent = HMAC+parser+token files; list-deployments = fetch+TLS SPKI pin; keychain-windows = EDR-sensitive; preflight = regex-normalized semver.

#### ENG DUAL VOICES — CONSENSUS TABLE

```
═══════════════════════════════════════════════════════════════
  Dimension                              Claude  Codex  Consensus
  ──────────────────────────────────────  ──────  ─────  ─────────
  1. Architecture sound?                   NO     NO    DISAGREE-w-plan
                                                        (source drift §1-3)
  2. Test coverage sufficient?             NO     NO    DISAGREE-w-plan
                                                        (mock surface)
  3. Performance risks addressed?          N/A    N/A   N/A (해당 없음)
  4. Security threats covered?             NO     NO    DISAGREE-w-plan
                                                        (위험 profile 잘못)
  5. Error paths handled?                  NO     PART  DISAGREE-w-plan
                                                        (encoding/symlink)
  6. Deployment risk manageable?           NO     NO    DISAGREE-w-plan
                                                        (MSVC/EDR/runner)
═══════════════════════════════════════════════════════════════
```

**5/6 dimension 에서 DISAGREE-w-plan.** 두 모델 합의: plan 이 imagined helper 를 review. 실제 source 와 align 시키려면 §1.1-§3 rewrite 필수.

### Step 1 — Architecture (수정된 ASCII Dependency Graph)

```
실제 module 의 위험 + Rust crate 매핑 (수정판):

┌───────────────────────────────────────────────────┐
│  axhub-helpers (Rust binary)                       │
│                                                    │
│  ┌─ index.rs (CLI dispatcher) ─┐                   │
│  │  clap + Bun.stdin.text 대체 │                   │
│  │  std::io::stdin().read()    │                   │
│  └────────┬────────────────────┘                   │
│           │                                        │
│   ┌───────┼───────┬───────────┬──────────┐         │
│   │       │       │           │          │         │
│  consent  list-   keychain   preflight  resolve   │
│  (HIGH)   deploy  (CRITICAL  (MED)      (MED)     │
│           (HIGH)   Win-EDR)                        │
│                                                    │
│  consent: hmac + sha2 + jsonwebtoken (HS256)       │
│           + nix(O_NOFOLLOW) + getrandom            │
│           + custom parser (regex bug-parity)       │
│                                                    │
│  list-deployments: rustls + tokio-rustls           │
│           + x509-parser + sha2 (SPKI pin)          │
│           + reqwest (rustls-tls feature)           │
│                                                    │
│  keychain: keyring crate AFTER spike +             │
│           PowerShell fallback (EDR safe)           │
│           Windows: native MSVC mandatory           │
│                                                    │
│  preflight: semver crate +                         │
│             regex (prerelease drop parity)         │
│                                                    │
│  resolve: reqwest + serde + dirs                   │
│  telemetry: tokio::fs + serde_json + chrono        │
│  redact: regex                                     │
│  catalog: serde_json + regex (codegen)             │
└───────────────────────────────────────────────────┘

External: keyring crate spike (1d) BEFORE Phase 2 commit
External: Windows MSVC native runner (matrix)
External: Korean EDR test cohort (V3/AhnLab) before Phase 4 ship
```

### Step 2 — Code Quality (DRY/naming/complexity)

- consent.ts 의 parseAxhubCommand (5-level recursion) → Rust 로 port 시 명확한 state machine 으로 재구현 권장. 현재 TS 가 regex backtracking 의존 — Rust `regex` crate 는 backtracking 없음 (linear time 보장). 일부 fixture 동작 달라질 수 있음. **bug-for-bug parity 또는 의도적 개선** 결정 필요.
- list-deployments.ts 의 SPKI pin set (HUB_API_SPKI_SHA256_PINS) → Rust 에서 `&[&str]` const array. 변경 불필요.
- keychain-windows.ts 의 PS_SCRIPT (53 lines inline C#) → Rust 에서 직접 `windows-rs::Win32::Security::Credentials::CredReadW` 호출 가능. 단, EDR 호환성 검증 후.

### Step 3 — Test Review (NEVER SKIP)

#### Test diagram (실제 codepaths)

| Module | New UX flow | Data flow | Codepath | Branch | Test type | Existing | Gap |
|--------|------------|-----------|----------|--------|-----------|----------|-----|
| consent.rs | login mint | HMAC-SHA256 sign + binding | `mintConsent → SignJWT` | exp=now, exp=now-1, exp=now+30 | unit + property | consent.test.ts (19KB) | clock=0 leeway lock test 없음 → **추가 필수** |
| consent.rs | verify | jwtVerify + binding eq | `verifyLatest → jwtVerify` | algorithm mismatch, key rotation | unit | consent.test.ts | 잘못된 algorithm header test 부재 → **추가** |
| consent.rs | parser | parseAxhubCommand 5-level recursion | wrap chars, env-var prefix | quoted args, unbalanced parens | fuzz | tests/fuzz-parser.ts | Rust regex 의 linear vs JS backtracking 차이 → **재작성** |
| consent.rs | token file | mode 0600 + O_NOFOLLOW | mintConsent file write | symlink attack, mode mismatch | integration | consent.test.ts | symlink-as-target test 부재 → **추가 필수** |
| list-deployments.rs | TLS pin | X509 SPKI hash | verifyHubApiTlsPin | pin mismatch, AXHUB_ALLOW_PROXY=1, timeout | integration | list-deployments.test.ts | proxy override test 누락 → **추가** |
| keychain-windows.rs | EDR detection | Bun.spawnSync + status code | runWindowsKeychain | 0xC0000409, AMSI block, V3 quarantine | platform | keychain-windows.test.ts | V3 cohort live test 없음 → **manual QA 필수** |
| preflight.rs | semver gate | regex prerelease drop + gte/lt | parseCliVersion | "1.2.3-rc.1", "1.2.3+build" | unit | preflight tests | prerelease drop bug-parity test 부재 → **추가** |

#### LLM/prompt 변경 — 해당 없음 (CLI helper)

#### Test 재작성 vs 단순 port 분석

- 단순 port 가능: redact, telemetry, catalog (JSON 기반, 의존성 0)
- 재작성 필요: consent.test.ts, list-deployments.test.ts, keychain*.test.ts, parser fuzz (bun:test mock 의존)
- 재작성 비용: **2~3주 추가** (plan §5.2 가 이를 가격 매기지 않음)

#### Test plan artifact

`/Users/wongil/.gstack/projects/jocoding-ax-partners-axhub/main-test-plan-20260429-113948.md` 에 별도 작성 (이 plan 끝에 첨부).

### Step 4 — Performance (해당 없음)

CLI helper, request-driven 단일 invocation. Performance critical path 부재. plan 의 cold start 주장은 dual voices 가 challenge — 사용자가 이를 알고 진행 결정.

### Phase 3 Completion Summary

| 항목 | 상태 | 비고 |
|------|------|------|
| Scope challenge (실제 code 검증) | ✓ | §1-3 source drift 발견 |
| ASCII architecture diagram | ✓ | 수정판 (위험 재배치) |
| Test diagram (codepath → coverage) | ✓ | 7 critical gap 식별 |
| Test plan artifact | ✓ | (별도 file) |
| "NOT in scope" | ✓ | (이미 §9) |
| "What already exists" | ✓ | (이미 §3.1-§5) |
| Failure modes registry | ✓ | (CEO phase + 보강) |
| Eng dual voices | ✓ | 5/6 DISAGREE-w-plan |
| Eng consensus table | ✓ | |

> **Phase 3 complete.** Codex: 8 concerns. Claude subagent: 10 issues. Consensus: 1/6 confirmed (5 DISAGREE-w-plan). 핵심 발견: plan 의 source-of-truth drift, Phase 3 위험 misstated, Windows MSVC 강제, 한국 EDR 보존 필수, test rewrite 비용 누락.

## Phase 3.5: DX Review

### Step 0 — DX Scope Assessment

**Product type:** CLI helper for Claude Code plugin (axhub deploy 도구). End-user 와 contributor 모두 영향.

**Developer journey 9-stage:**

| Stage | Current TS | After Rust port | Risk |
|-------|------------|-----------------|------|
| 1. Discover (`axhub` plugin install) | Bun runtime download | 변경 없음 (cosign binary) | 낮음 |
| 2. Install (`axhub update`) | curl + cosign verify | 변경 없음 (binary 만 교체) | **중** — 자동 마이그레이션 path 미정의 |
| 3. First command (`/axhub:deploy`) | helper bin 호출 | 변경 없음 | 낮음 |
| 4. Auth (`axhub:login`) | TS consent flow | Rust HMAC + token write | **높음** — go-keyring envelope 인터롭 미검증 |
| 5. Hello world (deploy 첫 시도) | 한글 에러 메시지 | Rust messages.rs (미설계) | **높음** — 해요체 drift |
| 6. Debug (`axhub:doctor`) | Bun version check | rustc check 추가? | **중** — SKILL 미수정 |
| 7. Upgrade (v0.1.x → v1.0) | minor bump 자동 | major bump (silent? opt-in?) | **높음** — UX 미정 |
| 8. Contribute (PR) | bun install + bun test | + cargo build (~5분 cold) + dual test | **매우 높음** — TTHW 6~8min |
| 9. Maintain (solo) | 1 runtime | 2 runtime 4개월 | **매우 높음** — 인지 부담 2배 |

### Step 0.5 — Dual Voices

#### CLAUDE SUBAGENT (DX — 독립 검토)

8 findings (평균 4.2/10):

- **F1 [CRITICAL] TTHW 2/10** — Cold-cache 6~8min, devcontainer/mise pin 없음, dual-runtime tax 미문서. **Fix:** `.tool-versions` (`rust 1.83.0` + `bun 1.1.0`) 추가, `bun run dev` umbrella, contributor docs Phase 1 deliverable
- **F2 [CRITICAL] Error messages 3/10** — 해요체 preservation hand-waved (§6 risk-table 한 줄). lint:tone 은 TS-only scanner. Rust `anyhow!()` 영어 leak 위험. **Fix:** Phase 1 deliverable — `scripts/check-toss-tone-conformance.ts` 가 `crates/**/*.rs` 도 scan + `messages.rs` 중앙화 + `msg!()` macro
- **F3 [HIGH] 마이그레이션 가이드 3/10** — Phase 4 ship 시 `axhub update` 자동 마이그레이션? token file format 호환? `axhub:doctor` SKILL Bun version 체크? 전부 미정의. **Fix:** Phase 1 deliverable — `docs/migrate-rust.md` skeleton, install.sh binary type 자동 감지, token-file format hard contract test
- **F4 [HIGH] Escape hatch 4/10** — `AXHUB_HELPERS_RUNTIME=rust` plan §5.2 에 E2E branching 으로만 언급. 사용자/contributor 문서 없음. **Fix:** first-class env var (`ts|rust|auto`, default `auto` until v1.0 stable) 문서화 (README + axhub:doctor)
- **F5 [HIGH] go-keyring envelope 4/10** — autoplan §H1 에서만 언급, Phase 1 prerequisite 아님. **Fix:** Phase 1 prerequisite 1-day spike — Rust `keyring` reads token written by Go `zalando/go-keyring`. 호환 안 되면 subprocess 유지
- **F6 [MED] skill workflow 7/10** — scripts/ 는 Bun 유지 (Rust port 영향 없음). 다만 plan §3.4 가 모호 ("plugin runtime 만 Bun 의존 유지"). **Fix:** scripts/ 는 Bun, helper binary 만 Rust — CLAUDE.md 명시
- **F7 [MED] nl-lexicon 8/10** — `lint:keywords` 는 SKILL.md 만 lock, Rust port 가 SKILL 안 건드리면 영향 없음. CI assertion 추가 trivial
- **F8 [MED] 문서 surface 4/10** — README, install.sh, install.ps1, axhub:doctor SKILL, CHANGELOG 전부 Bun 참조. inventory 없음. **Fix:** Phase 1 deliverable — `grep -r 'Bun\|bun ' docs/ skills/ install.* README.md` + Phase 4 checklist

#### CODEX SAYS (DX — adversarial)

**Verdict: weak DX for a solo-maintained 4-month dual-runtime migration. Developer-hostile in onboarding, parity, release safety, Korean UX.**

6 findings:
1. **TTHW 너무 느림** — 새 contributor 가 Bun + Rust + Cargo workspace + dual binaries + CI matrix + cosign release assumptions + 기존 plugin workflows 알아야. Vercel/Fly CLI 는 1언어 + 1 test command — 그게 표준
2. **Korean 해요체 underestimated** — messages.rs + anyhow + panic paths + Windows codepage + translated wrapper errors. Single Korean catalog + Rust-aware tone lint 없으면 drift
3. **axhub-cli backward compat red flag** — go-keyring-base64 envelope. `keyring` crate 단순화가 정확한 credential schema/OS attributes 호환 검증 안 하면 깨짐
4. **Docs Phase 4 늦음** — migration guide 가 Phase 1 artifact 여야 함. 호환성 약속 (env vars, keychain storage, update behavior, rollback, Bun 의존 잔여, platform 차이) 정의가 우선
5. **Upgrade path 심각한 hole** — axhub update + install.sh + install.ps1 + axhub:doctor 의 Bun 참조. 사용자가 broken doctor 또는 stale installer 로 runtime split 발견하게 둘 수 없음. compatibility matrix + preflight warning 필수
6. **Escape hatch first-class 아님** — `AXHUB_HELPERS_RUNTIME` 은 4개월 dual-runtime 기간 동안 first-class: rust/ts/auto + selected runtime 로깅 + rollback 지침 필수

**Adversarial conclusion:** Phase 1 시작 전 "DX contract" PR 추가 — one setup command, one parity test command, Rust-aware Korean tone lint, keyring interop spike, migration guide draft, installer/update/doctor audit, AXHUB_HELPERS_RUNTIME 문서화. 없으면 Rust port 가 internal rewrite 처럼 contributors + users 에 복잡성 leak.

#### DX DUAL VOICES — CONSENSUS TABLE

```
═══════════════════════════════════════════════════════════════
  Dimension                              Claude  Codex  Consensus
  ──────────────────────────────────────  ──────  ─────  ─────────
  1. Getting started < 5 min?              NO     NO    DISAGREE-w-plan
                                                        (TTHW 6~8min)
  2. API/CLI naming guessable?             YES    PART  CONFIRMED
                                                        (§9 freezes commands)
  3. Error messages actionable?            NO     NO    DISAGREE-w-plan
                                                        (해요체 hand-waved)
  4. Docs findable & complete?             NO     NO    DISAGREE-w-plan
                                                        (migration guide late)
  5. Upgrade path safe?                    NO     NO    DISAGREE-w-plan
                                                        (no auto-migration)
  6. Dev environment friction-free?        NO     NO    DISAGREE-w-plan
                                                        (dual-runtime tax)
═══════════════════════════════════════════════════════════════
```

**5/6 DISAGREE-w-plan.** 1 CONFIRMED (CLI freeze). 양 모델 합의: Phase 1 시작 전 "DX contract" 5개 deliverable 추가 필요.

### DX Implementation Checklist (Phase 1 prerequisite, 모두 1일 이내)

| # | 항목 | 비용 | 차단 효과 |
|---|------|------|-----------|
| DX-1 | `.tool-versions` (mise/asdf): rust 1.83.0 + bun 1.1.0 | 5 LOC | F1 — TTHW 단축 |
| DX-2 | `messages.rs` 중앙화 + `scripts/check-toss-tone-conformance.ts` 의 Rust scanner 확장 | ~150 LOC | F2 — 해요체 drift 차단 |
| DX-3 | `AXHUB_HELPERS_RUNTIME=ts\|rust\|auto` 문서화 (README + axhub:doctor SKILL update) | 30 LOC + SKILL edit | F4 — rollback path 보장 |
| DX-4 | Phase 1 spike: Rust `keyring` ⇄ Go `zalando/go-keyring` envelope 호환 | 1d spike | F5 — Phase 3 차단 해제 |
| DX-5 | `docs/migrate-rust.md` skeleton (Phase 1 작성, Phase 4 fill) | 200 LOC | F3 — Phase 4 안전 ship |
| DX-6 | Bun 참조 inventory: `grep -r 'Bun\|bun ' docs/ skills/ install.* README.md` → Phase 4 checklist | 1h | F8 — 문서 drift 차단 |

### TTHW Assessment

**Current (TS only):** ~15s cold (`bun install` + `bun run build`)
**During dual-runtime (Phase 1~3):** 6~8min cold (`+ rustup` + `+ cargo build --release`)
**Target post-port (Phase 4 단일 Rust):** ~2min cold (`cargo build --release` only, scripts/ 만 bun)

**TTHW target during Phase 1~3:** 3min (mise/asdf cache + cargo target dir cache + sccache 도입)

### Phase 3.5 Completion Summary

| 항목 | 상태 | 비고 |
|------|------|------|
| Developer journey map | ✓ | 9-stage, 5 단계가 high+ 위험 |
| DX dual voices | ✓ | 5/6 DISAGREE-w-plan |
| DX consensus table | ✓ | |
| DX scorecard (8 dimension) | ✓ | 평균 4.2/10 |
| DX Implementation Checklist | ✓ | 6 prerequisite |
| TTHW 평가 | ✓ | 6~8min during port |

> **Phase 3.5 complete.** DX overall: 4.2/10. TTHW: 6~8min → 3min target. Codex: 6 concerns. Claude subagent: 8 issues. Consensus: 1/6 confirmed (5 DISAGREE-w-plan). 핵심: 5개 DX prerequisite 가 Phase 1 시작 전 강제.

---

## Cross-Phase Themes

세 phase 양 모델이 독립적으로 발견한 high-confidence signals:

| Theme | CEO | Eng | DX | 강도 |
|-------|-----|-----|----|----|
| **Premise (통증 driver) 미검증** | ✓ ✓ | — | — | 매우 높음 |
| **Solo maintainer + bus factor 1** | ✓ ✓ | ✓ | ✓ | 매우 높음 |
| **Korean UX (해요체) 보존 risk** | — | ✓ | ✓ ✓ | 높음 |
| **go-keyring/keyring crate 인터롭 unverified** | — | ✓ ✓ | ✓ ✓ | 매우 높음 |
| **Test rewrite 비용 누락 (bun:test mock)** | — | ✓ ✓ | — | 높음 |
| **Windows MSVC + EDR 보존 필수** | — | ✓ ✓ | ✓ | 높음 |
| **Migration UX (axhub update + doctor)** | ✓ | — | ✓ ✓ | 높음 |
| **AXHUB_HELPERS_RUNTIME first-class** | — | — | ✓ ✓ | 중 |

**합치면:** plan 작성 후 두 모델 6개 phase 합쳐서 **18 critical/high finding**, 5/6 + 5/6 + 5/6 + 6/6 (CEO) = 21/24 dimension 에서 plan 과 disagree. Plan 자체의 §1-3 source drift 가 root cause — 작성자 (나) 가 grep 없이 추정으로 inventory 작성.

---

## Phase 4: Final Approval Gate

### Plan Summary

**제안:** axhub-helpers (TypeScript, 2,536 LOC, 10 모듈, Bun runtime) → Rust 4단계 점진 포팅 (8~12주, solo maintainer). Binary 5~10x 축소, cold start 5x, memory -80% 주장.

### Decisions Made

- **Auto-decided:** 0 (모든 review section 이 USER CHALLENGE 또는 plan-level mandatory mitigation 으로 surface)
- **Taste choices:** 6 (DX prerequisite checklist DX-1~DX-6)
- **User challenges:** 2 (Phase 1 premise gate + Phase 3.5 DX hole)

### User Challenges (양 모델 합의가 사용자 stated direction 과 disagree)

#### Challenge 1: Plan 시작 전 1주 validation sprint (premise 검증)
- **사용자가 말한 것:** "ts로 구성된거 전부 다 rust로 완벽 포팅할꺼야"
- **양 모델 권장:** 1주 validation sprint 먼저 (hyperfine 측정 + Bun 최적화 시도 + 사용자 통증 audit)
- **이유:** premise (통증 driver) 미검증, 8~12주 opportunity cost 가격 미매김, 6개월 후 regret 시나리오 plausible
- **놓칠 수 있는 context:** 사용자의 internal product priority, 학습 목표, 보안 요구사항, 시장 타이밍
- **틀리면 비용:** 8~12주 솔로 노력 손실 + Q3 기능 velocity 손실 + 사용자가 신경 안 씀
- **사용자 결정 (premise gate):** **REJECTED** — full Rust 포팅 진행 결정 (이미 Phase 1 에서 완료)

#### Challenge 2: Phase 1 시작 전 6개 DX prerequisite 강제
- **사용자가 말한 것:** "rust로 완벽 포팅" — DX prerequisite 명시 안 함, 즉시 시작 의사
- **양 모델 권장 (Eng + DX):** Phase 1 commit 전 DX-1~DX-6 prerequisite 완료
- **이유:** TTHW 6~8min, 해요체 drift 차단, axhub-cli 호환 검증, escape hatch, migration guide
- **놓칠 수 있는 context:** 사용자가 contributor 받을 의사 없을 수도 (solo 만 — 이러면 TTHW 영향 작음), migration guide 우선순위 사용자만 결정 가능
- **틀리면 비용:** Phase 1~3 의 4개월 dual-runtime 기간 사용자 본인이 매일 마찰 + Phase 4 ship 시 사용자가 token 잃음 (go-keyring 인터롭 깨지면)
- **권장:** DX-1, DX-2, DX-4 는 강제 (해요체 + go-keyring + escape hatch). DX-3, DX-5, DX-6 는 사용자 결정.

### Auto-Decided

(없음 — review 가 모두 high-stakes 라 surface)

### Review Scores

- **CEO:** 6/6 dimension DISAGREE-w-plan (premise 미검증). User REJECTED challenge — proceed.
- **CEO Voices:** Codex 8 concerns, Claude subagent 7 issues, Consensus 0/6 confirmed
- **Design:** SKIPPED — no UI scope (5 매치 false positive: Rust module/platform 맥락)
- **Eng:** 5/6 DISAGREE-w-plan. plan §1-3 source drift critical, 위험 profile 재배치 필요.
- **Eng Voices:** Codex 8 concerns, Claude subagent 10 issues, Consensus 1/6 confirmed
- **DX:** 5/6 DISAGREE-w-plan. 평균 4.2/10. 6 prerequisite 식별.
- **DX Voices:** Codex 6 concerns, Claude subagent 8 issues, Consensus 1/6 confirmed

### Cross-Phase Themes (위 표 참조)

3 phase 모두에서 독립 발견된 핵심: solo maintainer + bus factor 1, Korean UX 보존 risk, go-keyring 인터롭, Windows MSVC + EDR.

### Deferred to TODOS.md

- Plugin runtime Bun 제거 (별도 plan)
- 12-month ideal 의 install < 5초 (별도 work)

### Mandatory Plan Corrections (양 모델 합의로 surface)

Plan 진행 전 다음 수정 필수 (사용자 결정 후에도 안 빼는):

1. **§1.1 inventory 수정** — consent.ts 의 외부 의존을 `jose (HS256 only) + crypto.randomBytes/randomUUID + nix(O_NOFOLLOW)` 로. TLS+X509 표시는 list-deployments.ts 행으로 이동
2. **§1.2 매핑 수정** — `consent.rs: hmac+sha2+jsonwebtoken+nix+getrandom`, `list-deployments.rs: rustls+tokio-rustls+x509-parser+sha2+reqwest`
3. **§3.3 Phase 3 정의 수정** — 위험 profile 재배치 (consent 는 parser+file-mode+HMAC lifecycle, mTLS 아님)
4. **§4.2 Windows target 수정** — `x86_64-pc-windows-gnu` → `x86_64-pc-windows-msvc` (Authenticode + EDR + signtool)
5. **§5.2 test parity 수정** — bun:test mock 30~50% 재작성 비용 명시 (+2~3주)
6. **§6 risks 추가** — JWT leeway 0 lock, parser linear-time vs backtracking 차이, 한국 EDR 보존, MSVC 강제, go-keyring 호환 spike
7. **Phase 1 prerequisite 추가:** DX-1 (.tool-versions), DX-2 (messages.rs + Rust tone lint), DX-4 (keyring spike)
8. **현실 추정 update:** 8~12주 → 10~14주 (test rewrite + DX prerequisite + MSVC native runner)

---

## Phase 1 Prerequisite Checklist (사용자 승인 — 시작 전 강제)

다음 모두 완료 후 Phase 1 (Foundation: redact/catalog/telemetry) 시작 가능. 각 항목 1일 이내.

### Mandatory (Phase 1 commit 차단)

- [ ] **DX-1: `.tool-versions` 추가** — `rust 1.83.0` + `bun 1.1.0`. mise/asdf 호환. 5 LOC.
- [ ] **DX-2: messages.rs 중앙화 + Rust tone lint** — `crates/axhub-helpers/src/messages.rs` 빈 catalog + `scripts/check-toss-tone-conformance.ts` 가 `crates/**/*.rs` 도 scan (`anyhow!`, `format!`, `.context()` regex). ~150 LOC.
- [ ] **DX-3: AXHUB_HELPERS_RUNTIME 문서화** — README + `axhub:doctor` SKILL update. `ts|rust|auto` (default `auto` until v1.0 stable).
- [ ] **DX-4: keyring crate ⇄ go-keyring envelope 호환 spike** — 1일 spike, axhub-cli 가 write 한 token 을 Rust `keyring::Entry::get_password()` 로 read. 3 OS 모두 검증. **호환 안 되면 subprocess (security/secret-tool/PowerShell) 유지** 결정.
- [ ] **사전 정정: source 검증** — `consent.ts` JWE 사용 grep (예상: 0건), `Bun\.` API survey (이미 5건 확인), `tls.connect`/`X509Certificate` 위치 확인 (이미 list-deployments.ts 확인됨)
- [ ] **사전 정정: JWT leeway 0 lock test** — `consent.ts` 의 jwtVerify 가 leeway 0 으로 동작 검증하는 test 추가 (`exp = now - 1` MUST fail). 이게 TS 측에서 먼저 lock 되어야 Rust port 가 silent widen 안 함.

### Recommended (Phase 1 시작은 가능, Phase 4 ship 전 강제)

- [ ] **DX-5: `docs/migrate-rust.md` skeleton** — Phase 1 작성, Phase 4 fill. 호환성 약속 (env vars, keychain storage, update behavior, rollback) 정의.
- [ ] **DX-6: Bun 참조 inventory** — `grep -r 'Bun\|bun ' docs/ skills/ install.* README.md` 결과 → Phase 4 checklist.

### CI 추가

- [ ] Rust matrix 추가 (block 안 함, 정보용) — Phase 1 PR 마다 `cargo build` + `cargo test`
- [ ] `lint:keywords --check` 가 모든 Rust PR 에서 SKILL.md frontmatter 변경 차단
- [ ] Windows MSVC native runner 매트릭스 추가 (windows-latest)

---

## /autoplan 결과 — Approved as-is (2026-04-29)

**User decision:** 모든 review correction 수용 후 Phase 1 시작. Premise gate 는 user sovereignty 로 진행 결정 유지 (양 모델 권장 1주 validation sprint 대신).

**Plan 본문 정정 적용 완료:**
- §1.1 inventory: consent.ts 행 의존성 수정, list-deployments.ts 행 위험 매우 높음
- §1.2 매핑: TLS+X509 가 list-deployments 소속 명시, JWE 미사용 명시, jose JWS-only 확인
- §4.2 Windows: GNU → MSVC 강제, 근거 명시
- §1 끝: 정정 note 추가 (consent 가 mTLS 아님)

**Plan 의 본문 §3.3, §5.2, §6 은 cross-phase summary 와 mandatory correction 섹션에서 이미 충분히 정정됨 — duplicate edit 생략.**

**현실 추정:** 10~14주 (test rewrite + DX prerequisite + MSVC native runner 반영).

**Next step:** Phase 1 prerequisite checklist 위 8 mandatory 항목 진행 → 완료 시 Phase 1 (Foundation) 시작 → ADR `.omc/adr/0001-rust-port-decision.md` 작성.

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/autoplan` | Scope & strategy | 1 | issues_open | 7 issues / 8 concerns (premise 미검증) — user REJECTED challenge |
| Codex Review | `codex exec` (CEO+Eng+DX) | Independent 2nd opinion | 3 | issues_open | 22 concerns total (8+8+6) |
| Eng Review | `/autoplan` Phase 3 | Architecture & tests | 1 | issues_open | 10 issues (5/6 DISAGREE-w-plan, source drift) |
| Design Review | `/autoplan` Phase 2 | UI/UX gaps | 0 | skipped | No UI scope |
| DX Review | `/autoplan` Phase 3.5 | Developer experience | 1 | issues_open | 8 issues (avg 4.2/10, 5/6 DISAGREE) |

**VERDICT:** APPROVED-WITH-MANDATORY-CORRECTIONS. 사용자가 양 모델의 STOP 권장에 user sovereignty 행사. Phase 1 prerequisite 8 mandatory + plan correction 적용. 현실 추정 10~14주.









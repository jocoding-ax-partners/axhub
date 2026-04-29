# Test 전략

**현재 baseline:** 428 `test()` + 499 `test/describe` (Phase 18, bun test)
**Rust port 후 목표:** 380+ cargo test + 24h cargo-fuzz parser + V3/AhnLab cohort QA

---

## 1. Test 분류

### 1.1 단순 port 가능 (의존성 0)

| 모듈 | TS test | Rust 매핑 | 비용 |
|------|---------|-----------|------|
| redact | `redact.test.ts` | `#[test]` 직접 | 0.5일 |
| catalog | corpus parity | `classify_parity_*` test | 1일 |
| telemetry | envelope schema | `#[tokio::test]` | 0.5일 |

**총:** 2일

### 1.2 부분 port (fixture 재사용 + mock 일부 재작성)

| 모듈 | TS test | 재사용 가능 | 재작성 필요 |
|------|---------|-------------|-------------|
| preflight | `preflight.test.ts` | 버전 파싱 fixture | spawn mock (`Bun.spawnSync` stub) |
| resolve | profile fixtures | env vs file 우선순위 | XDG_CONFIG_HOME edge cases |

**총:** 1주

### 1.3 재작성 필요 (bun:test mock 의존)

| 모듈 | TS test | 재작성 사유 | 비용 |
|------|---------|-------------|------|
| list-deployments | `list-deployments.test.ts` | TLS mock + bun:test spy | 1주 |
| consent | `consent.test.ts` (19KB) | mock + parser fuzz + Korean wording | 1.5주 |
| keychain | `keychain.test.ts`, `keychain-windows.test.ts` | platform subprocess mock | 1주 |
| main (index 대응) | `axhub-helpers.test.ts` | clap dispatch + e2e | 1주 |

**총:** 4.5주 (=> Phase 4 까지 분산)

### 합계

- 단순 port: 2일
- 부분 port: 1주
- 재작성: 4.5주
- **=> 약 6주가 test 작업 단독.** Phase 1~4 에 분산되지만, 단순한 "Rust tests + JSON fixtures" 매핑보다 훨씬 비쌈.

이 비용은 plan §5.2 의 "498 test parity" 주장이 가격 미매김. /autoplan 의 추정 update (8~12주 → 10~14주) 의 일부.

---

## 2. Codepath Gap (Eng review 발견)

각 Rust 모듈에 새로 추가해야 할 test (TS 에 없거나 부족):

| Module | Codepath | Branch | Gap | 추가할 test |
|--------|----------|--------|-----|------------|
| consent.rs | mintConsent → SignJWT | exp=now / exp=now-1 / exp=now+30 | clock=0 leeway lock 없음 | `rejects_expired_with_zero_leeway` |
| consent.rs | verifyLatest → jwtVerify | algorithm mismatch / key rotation | 잘못된 alg header test 부재 | `rejects_algorithm_swap` |
| consent.rs | parseAxhubCommand 5-level recursion | wrap chars / env-var prefix | Rust regex linear vs JS backtracking 차이 | parser fuzz target (cargo-fuzz 24h) |
| consent.rs | mintConsent file write | symlink attack / mode mismatch | symlink-as-target test 부재 | `refuses_symlink_target`, `refuses_world_readable` |
| list-deployments.rs | verifyHubApiTlsPin | pin mismatch / AXHUB_ALLOW_PROXY=1 / timeout | proxy override test 누락 | `proxy_override_skips`, `timeout`, `non_pinned_host_skipped` |
| keychain-windows.rs | runWindowsKeychain | 0xC0000409 / AMSI block / V3 quarantine | V3 cohort live test 없음 | manual QA matrix (자동화 안 됨) |
| preflight.rs | parseCliVersion | "1.2.3-rc.1" / "1.2.3+build" | prerelease drop bug-parity 부재 | `drop_prerelease`, `drop_build_metadata` |

---

## 3. Fixture 호환성

### JSON 기반 (재사용 가능)

- `tests/corpus.jsonl` (24KB)
- `tests/corpus.100.jsonl`
- `tests/corpus.20.jsonl`
- `tests/fixtures/mock-hub/*.json`
- `tests/fixtures/profiles/*.json`
- `tests/fixtures/preflight/*.json`
- `tests/fixtures/ask-defaults/registry.json`

### bun:test mock (재작성 필요)

- `mock(...)` calls in consent.test.ts
- `spyOn(...)` calls in list-deployments.test.ts
- module-mocking via `import.meta.require`

### Bun-specific compiled binary contract

- `tests/auto-download.test.sh` (binary download flow)
- `tests/install.test.sh`
- `tests/install-ps1.test.ts`
- `tests/live-plugin-smoke.sh`

이들은 binary 산출물에 대한 contract — Rust binary 도 동일 contract 만족하면 OK. 단, binary path / 사이즈 / extension assertion 일부는 변경 필요.

### Korean wording assertions

- `tests/lint-toss-tone.test.ts` — TS 측 baseline. Rust 측은 별도 `lint:tone:rust` script.
- `tests/fixtures/skill-doctor/*` — SKILL.md 한글 frontmatter assertion. Rust port 영향 없음.

---

## 4. cargo-fuzz parser (24h, 필수)

```bash
# Phase 3 진입 시 nightly CI 에 추가
cargo fuzz init
cargo fuzz add parser

# fuzz/fuzz_targets/parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use axhub_helpers::consent::parser::parse_axhub_command;

fuzz_target!(|input: &str| {
    let _ = parse_axhub_command(input);  // panic 없어야 함
});

# 24h run (nightly only)
cargo fuzz run parser -- -max_total_time=86400
```

**Pass criteria:** 24h 무 crash, 무 panic.

---

## 5. V3/AhnLab Live Cohort QA (Phase 3 + Phase 4 강제)

**자동화 어려움.** Manual QA matrix:

| OS | EDR | Test |
|----|-----|------|
| Windows 10 Korean | V3 (AhnLab) | `axhub:login` + `axhub:deploy` 한 사이클 차단 없이 |
| Windows 10 Korean | CrowdStrike Falcon | 동일 |
| Windows 11 Korean | AhnLab Smart Defense | 동일 |
| Windows 11 Korean | Symantec | 동일 (대조군) |

**Pass criteria:**
- 모든 EDR 에서 `keychain.read` 성공
- AMSI block 발생 시 `EDR_BLOCKED` 한글 에러 메시지 표시
- 0xC0000409 status code 정확 처리

**실행 시기:**
- Phase 3 완료 시 1차 (keychain.rs ship 전)
- Phase 4 완료 시 2차 (full integration)
- 매 release (mandatory)

---

## 6. CI Matrix

```yaml
# .github/workflows/rust-ci.yml
name: rust-ci
on: [pull_request, push]

jobs:
  unit:
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - run: cargo test --target ${{ matrix.target }}
  
  integration:
    runs-on: ubuntu-latest
    services:
      mock-hub:
        image: axhub/mock-hub:latest
        ports: [8080:8080]
    steps:
      - run: cargo test --test '*' --target x86_64-unknown-linux-gnu
  
  fuzz-nightly:
    if: github.event.schedule == '0 0 * * *'
    runs-on: ubuntu-latest
    steps:
      - run: cargo fuzz run parser -- -max_total_time=86400
  
  tone-lint:
    runs-on: ubuntu-latest
    steps:
      - run: bun run lint:tone:rust --strict
  
  skill-frozen:
    runs-on: ubuntu-latest
    steps:
      - run: bun run lint:keywords --check
  
  parity-mapping:
    runs-on: ubuntu-latest
    steps:
      - name: check parity table present
        run: |
          # PR 마다 crates/axhub-helpers/tests/parity-*.md 가 있어야 함
          for module in redact catalog telemetry preflight resolve list_deployments consent keychain; do
            test -f "crates/axhub-helpers/tests/parity-${module}.md" || exit 1
          done
```

---

## 7. Phase 별 Test Exit Criteria

### Phase 1
- [ ] redact / catalog / telemetry cargo test PASS
- [ ] corpus parity 100% match (TS classify == Rust classify)

### Phase 2
- [ ] preflight semver bug-for-bug parity
- [ ] list-deployments TLS pin 19 case
- [ ] AXHUB_ALLOW_PROXY 옵트아웃 검증

### Phase 3
- [ ] consent: HMAC key + JWT mint+verify + parser fuzz 24h
- [ ] symlink defense + world-readable 거부
- [ ] keychain: 3 OS read + go-keyring envelope 양방향
- [ ] V3/AhnLab cohort QA 1차 통과

### Phase 4
- [ ] e2e matrix t1/t2/nightly 100%
- [ ] V3/AhnLab cohort QA 2차 통과
- [ ] Bun 참조 inventory 100% 처리
- [ ] 1주 monitor 기간 회귀 0건

---

## 8. Test 회귀 ledger

매 phase 별 PR 에 첨부:

```markdown
## Test Parity Mapping

| TS test | Rust test | Status | 비고 |
|---------|-----------|--------|------|
| consent.test.ts:42 mintConsent basic | crates/.../consent/jwt.rs:tests::mint_verify_round_trip | ✓ MATCH | |
| consent.test.ts:78 expired token | crates/.../consent/jwt.rs:tests::rejects_expired_with_zero_leeway | ✓ MATCH | leeway 0 lock 추가 |
| consent.test.ts:120 parser quoted | crates/.../consent/parser.rs:tests::quoted_args | △ DIFFERENT | state machine 재구현, fixture #5 결과 변경 (의도적) |
| consent.test.ts:180 nonexistent feature | — | ✗ DROPPED | TS 측 mock 의존 → Rust 에서 의미 없음 |
```

PR template 에 강제 — `parity-{module}.md` 가 있어야 merge 가능.

---

## 9. Honest Tradeoff (CHANGELOG 에 명시)

```
### Test baseline change
- 498 → 480 (TS) + 380 (cargo test) + 24h fuzz
- 18 case (TS-mock-only) 가 Rust state machine 구현으로 무효화
- parser fixture #5, #12 가 backtracking-의존 → state machine 결과 다름 (의도적)
- V3/AhnLab cohort QA 가 manual (자동화 안 됨, 매 release 마다 필수)
```

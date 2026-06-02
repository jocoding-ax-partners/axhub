---
description: "Task list — Skill 복구 라우팅 ↔ 현행 CLI 실패 신호 정합"
---

# Tasks: Skill 복구 라우팅 ↔ 현행 CLI 실패 신호 정합

**Input**: `specs/004-skill-exit-code-alignment/` (plan.md · spec.md · research.md · data-model.md · contracts/cli-error-envelope.md · quickstart.md · verification-report.md)

**Tests**: 포함해요 — spec FR-012/SC-008/Q2 가 drift-guard parity 테스트를 명시 deliverable 로 요구하고, exit→slug 매핑 단위테스트도 회귀 보호로 필요해요.

**Organization**: user story 별 단계. ⚠ **공유 파일 제약**: `data/catalog.json` · `error-empathy-catalog.md` · `list_deployments.rs` · `main.rs` 는 여러 스토리가 같은 파일을 편집해요 → 스토리 간 이 파일 작업은 **순차**(같은 파일 [P] 금지). 스토리는 독립 *테스트*는 가능하나 독립 *병렬 머지*는 공유 파일 때문에 제한돼요.

## Format: `[ID] [P?] [Story] Description (file)`

- **[P]**: 다른 파일 + 미완 의존 없음일 때만
- **[Story]**: US1~US6 (spec 매핑); Setup/Foundational/Polish 는 라벨 없음

## Path Conventions

axhub 플러그인 단일 repo: Rust helper `crates/axhub-helpers/`, 카탈로그 source `crates/axhub-helpers/data/`, codegen `scripts/`, 스킬 `skills/`, 테스트 `tests/` + `crates/axhub-helpers/src/` 인라인.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 작업 전 baseline 확보 (회귀 기준점).

- [X] T001 prebuilt `ax-hub-cli/target/debug/axhub` 존재·실행 확인 (`axhub --version` = 0.17.2). 없으면 `cargo build -p axhub` 안내 메모를 `research.md` §남은 gate-zero 에 추가
- [ ] T002 green baseline 캡처: `cargo test`(axhub-helpers) + `bun test`(전체) + `bunx tsc --noEmit` 결과를 `specs/004-skill-exit-code-alignment/research.md` 하단에 baseline 카운트로 기록
- [ ] T003 [P] `git -C ax-hub-cli status` clean 확인 (CLI 미수정 Out-of-Scope 보증 기준점)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: canonical 매핑·slug 확정·pinned snapshot — 모든 스토리가 소비해요.

**⚠️ CRITICAL**: T004~T011 완료 전 어떤 스토리 편집(catalog/helper)도 시작 금지 (data-model B/C 표가 미확정이면 잘못된 키로 편집됨).

- [X] T004 [P] auth(4) slug 문자열 확정: deauth 후 인증필요 명령 `--json` live 실행 또는 `crates/axhub-core/src/error.rs Error::code()` 전수로 `unauthenticated` 확인 → `data-model.md` A표 "추정" 제거
- [X] T005 [P] `rate_limited`(6)·`tenant_*`(8)·`timeout`(10) 등 미관찰 slug 확정 (`error.rs` `Error::code()`/`subcode()` 전수) → `data-model.md` A표 갱신
- [X] T006 [P] `apis.call_consent_required` 실제 출처 확정 (ax-hub-cli grep 0건 — plugin-side `crates/axhub-helpers/` 또는 stale 판정) → `data-model.md` B표 해당 행 결론
- [ ] T007 reachability 매핑: `8`~`15` 각 코드가 어느 skill 경로에 실제 도달하는지 (deploy/status/recover/auth/init/apps/invitations) 표로 → `data-model.md` B표 "신규 추가" 의 bespoke vs fallback 대상 확정 (Q1)
- [X] T008 `EXIT_LIST_AUTH(65)`/`EXIT_LIST_NOT_FOUND(67)` helper-exit **소비처** 전수: `rg -rn 'list-deployments' skills/` + helper-exit 숫자에 의존하는 skill 라우팅 식별 → `data-model.md` D표 각주 갱신
- [X] T009 CLI 실패 계약 pinned snapshot 생성: `crates/axhub-helpers/data/cli-exit-contract.json` (T004~T005 의 `{code, exit_code, subcode}` 전집합) — `contracts/cli-error-envelope.md` 와 일치. **T006 의 `apis.call_consent_required` 출처 결론 반영 후 확정** (plugin-side 판정 시 CLI 계약 모집단에서 제외 — U2)
- [X] T010 flat↔dotted slug **단일 변환점** 설계 확정 (CLI flat slug 을 canonical 1차 키로; helper dotted 은 1곳에서 변환) → `data-model.md` C표 최종화
- [X] T011 data-model.md B/C 표를 최종 canonical 매핑으로 잠금 (이후 스토리는 이 표만 적용) — **A0-5 의 3-surface 변경집합이 정본**

**Checkpoint**: 매핑·snapshot 확정 — 스토리 편집 시작 가능.

---

## Phase 3: User Story 1 — 로그인 만료 → 재로그인 (Priority: P1) 🎯 MVP

**Goal**: 토큰 만료(CLI exit 4 / slug `unauthenticated`)가 재로그인 안내로 라우팅. status 가 user 진입점. (slug 문자열은 **T004 확정값** 사용 — `unauthenticated` 은 추정 placeholder.)

**Independent Test**: 만료 토큰(또는 `unauthenticated`/exit 4 주입)으로 `status` 실행 → 일반 통신-오류 아닌 재로그인 안내; 비대화형은 등록 기본값(abort). (quickstart §1)

### Tests for User Story 1 ⚠️

- [X] T012 [P] [US1] exit→slug 단위테스트: `4 → unauthenticated`, helper-exit auth 분기 in `crates/axhub-helpers/src/list_deployments.rs` (#[cfg(test)] mod) — 현재 RED
- [ ] T013 [P] [US1] catalog 에 `unauthenticated`(auth) 항목 존재 + 옛 `65` base 부재 단언 in `tests/exit-contract-parity.test.ts` (auth 부분)

### Implementation for User Story 1

- [X] T014 [US1] `crates/axhub-helpers/data/catalog.json`: `65` → `unauthenticated`(+/`4` 2차) 재키 (data-model B표)
- [X] T015 [US1] `crates/axhub-helpers/src/list_deployments.rs`: `EXIT_LIST_AUTH`/`exit_to_error_code`(65→4 + slug)/`exit_to_helper_exit`(`starts_with("auth.")` → `=="unauthenticated" ‖ exit==4` 도 매치) (data-model C표)
- [ ] T016 [US1] `crates/axhub-helpers/src/main.rs` `cmd_classify_exit` + :1981 하드코딩 `65=>"auth 만료"` → slug/`4` 기준
- [X] T017 [US1] `skills/status/SKILL.md` Step 7 의 `exit 65 → token expired` → CLI slug(`unauthenticated`)/`4` 기준 라우팅 (D1 비대화형 가드의 코드 언급 포함)
- [X] T018 [US1] `skills/deploy/references/error-empathy-catalog.md` auth 섹션 `## exit 65` → 신 키 + `contracts/cli-error-envelope.md` 인용 (FR-011)

**Checkpoint**: 만료-토큰 → 재로그인 동작 (MVP). T012 GREEN.

---

## Phase 4: User Story 2 — 리소스 못 찾음 → did-you-mean (Priority: P1)

**Goal**: not-found(CLI exit 5 / slug `not_found`)가 did-you-mean 으로 라우팅.

**Independent Test**: 없는 앱 참조 → 가까운 후보 제시 (일반 실패 아님). (quickstart §2)

### Tests for User Story 2 ⚠️

- [X] T019 [P] [US2] exit→slug 단위테스트 `5 → not_found` + helper-exit notfound 분기 in `crates/axhub-helpers/src/list_deployments.rs`

### Implementation for User Story 2

- [ ] T020 [US2] `crates/axhub-helpers/data/catalog.json`: `67`→`not_found`/`5` + subcodes `67:{github.install_not_found,open.no_app_manifest,catalog.not_found}` → `5:<sub>` (data-model B표) — T014 와 같은 파일이라 순차
- [X] T021 [US2] `crates/axhub-helpers/src/list_deployments.rs`: notfound 매핑 flat `not_found` 정합 (skill-level 키 일치) — T015 와 같은 파일 순차
- [ ] T022 [P] [US2] `skills/logs/SKILL.md` `exit 67 → deploy id not found` → slug 기준
- [X] T023 [US2] `skills/status/SKILL.md` not-found 경로 + helper `list-deployments` did-you-mean 정합 (T017 와 같은 파일 순차)
- [X] T024 [US2] `skills/deploy/references/error-empathy-catalog.md` not-found 섹션 재키 + 인용 (T018 와 같은 파일 순차)

**Checkpoint**: not-found → did-you-mean 동작.

---

## Phase 5: User Story 3 — 정책 차단 vs 권한 부족 구분 (Priority: P1)

**Goal**: `66`(EnforceBlocked: downgrade/cosign) 와 일반 권한-부족이 키 충돌 없이 분리. base `66="scope insufficient"` 재정의.

**Independent Test**: 정책-차단·권한-부족 각각 주입 → 서로 다른 고유 템플릿 (키 공유 0). (quickstart §2 parity)

### Tests for User Story 3 ⚠️

- [ ] T025 [P] [US3] 충돌 단언: 어떤 catalog 키도 두 의미로 매핑 안 됨 + `66` base 가 EnforceBlocked 만 in `tests/exit-contract-parity.test.ts`

### Implementation for User Story 3

- [ ] T026 [US3] `crates/axhub-helpers/data/catalog.json`: `66` base "scope insufficient" 재정의(EnforceBlocked 전용), `66:scope.downgrade_blocked`/`66:update.cosign_verification_failed` 유지, `66:profile.endpoint_not_in_allowlist` 재분류(→`64` usage 계열, data-model B표) — T014/T020 와 같은 파일 순차
- [ ] T027 [US3] 일반 권한-부족(tenant scope `8`) 별도 키 신설 + `skills/init/SKILL.md` `exit 66 (forbidden/scope)` 정합 (init 은 이미 `exit 8` 사용 — 통일)
- [ ] T028 [US3] `skills/deploy/references/error-empathy-catalog.md` `66` 섹션 재정의 + 인용 (T018/T024 와 같은 파일 순차)

**Checkpoint**: 정책-차단 vs 권한-부족 분리 (충돌 0).

---

## Phase 6: User Story 4 — rate limit → 자동 backoff (Priority: P2)

**Goal**: rate-limit(CLI exit 6 / slug `rate_limited`)가 Retry-After backoff 로 라우팅. (slug 문자열은 **T005 확정값** 사용 — `rate_limited` 은 추정 placeholder.)

**Independent Test**: rate-limit(재시도-지연 포함) 주입 → 대기·안내. (quickstart §2)

### Tests for User Story 4 ⚠️

- [ ] T029 [P] [US4] exit→slug 단위테스트 `6 → rate_limited` in `crates/axhub-helpers/src/list_deployments.rs`

### Implementation for User Story 4

- [ ] T030 [US4] `crates/axhub-helpers/data/catalog.json`: `68`→`rate_limited`/`6` 재키 + helper 에 rate_limited 분기 추가 (data-model B/C) — 공유 파일 순차
- [ ] T031 [P] [US4] `skills/logs/SKILL.md` `exit 68 → rate limit` → slug 기준 (logs = 최다 rate-limited 표면)
- [ ] T032 [US4] `skills/status/SKILL.md` + `error-empathy-catalog.md` rate-limit 정합 (공유 파일 순차)

**Checkpoint**: rate-limit → backoff 동작.

---

## Phase 7: User Story 5 — 미처리 실패 조건 안전 fallback (Priority: P2)

**Goal**: catalog 항목 없는 CLI 조건이 정직한 fallback 으로 (엉뚱한 원인 아님). exit-2 제거, `70`→`7`.

**Independent Test**: 전용 템플릿 없는 조건 주입 → "안전하게 멈췄어요 + 다음 행동"; 잘못된-원인 0. (quickstart §1)

### Tests for User Story 5 ⚠️

- [ ] T033 [P] [US5] fallback 단언: 미매핑 CLI 코드 → 공통 fallback (generic 아님) + exit-2 키 부재 in `tests/exit-contract-parity.test.ts`

### Implementation for User Story 5

- [ ] T034 [US5] `crates/axhub-helpers/data/catalog.json`: `2` 제거(clap 예약), `70:catalog.internal_error`→`7` 계열 재키 — 공유 파일 순차
- [ ] T035 [US5] T007 reachability 의 reachable 코드(예 `8`/`9`/`10`/`12`/`13`)에 bespoke 4-part 항목 추가 in `crates/axhub-helpers/data/catalog.json`
- [ ] T036 [US5] 공통 "안전하게 멈췄어요" fallback 항목 1개 + helper catch-all(`cli.exit_<N>`) 정합 in `crates/axhub-helpers/src/list_deployments.rs` (공유 파일 순차)

**Checkpoint**: 미처리 조건 안전 fallback. 모든 CLI 코드가 정확히 1항목 (SC-001).

---

## Phase 8: User Story 6 — 카탈로그 단일 출처 정합 + drift 가드 (Priority: P1, 횡단)

**Goal**: 8 skill 이 같은 조건을 동일 라우팅 + 재발 방지 가드. codegen 동기.

**Independent Test**: `rg 'exit 6[5-8]|exit 70' skills/` = 0; parity 가드 fail-on-drift 작동. (quickstart §3,§5)

- [X] T037 [US6] `scripts/codegen-catalog.ts` 재실행 → `error-empathy-catalog.generated.md` 재생성 + `tests/codegen.test.ts` GREEN (hand-written ↔ generated 키 동기)
- [X] T038 [US6] **drift-guard parity 테스트** in `tests/exit-contract-parity.test.ts`: catalog 키 ⊆ `data/cli-exit-contract.json`, 미지 키 0, 양방향 커버 (FR-012/SC-008/Q2)
- [ ] T039 [P] [US6] `skills/recover/SKILL.md` canonical helper-route map(dotted slug) ↔ CLI flat slug 정합 + 잔여 `exit 65` 정리
- [ ] T040 [P] [US6] `skills/apps/SKILL.md` 잔여 numeric(`65/67/68`) → slug + preflight `auth_error_code` 정합
- [ ] T041 [P] [US6] `skills/init/SKILL.md` 잔여 `65`/`66` → slug/`4`/`8` (이미 쓰는 `exit 8` 과 통일)
- [ ] T042 [US6] 전 skill sweep: `rg -n 'exit 6[5-8]|exit 70|"6[5-8]"' skills/` 0건 확인 + 각 항목 `contracts/cli-error-envelope.md` 인용 (FR-011/SC-005/SC-007)
- [ ] T042b [US6] **FR-008 커버리지**: 8 skill 의 D1 비대화형 가드(`! [ -t 1 ] || $CI || $CLAUDE_NON_INTERACTIVE`)가 신 slug/코드로 `tests/fixtures/ask-defaults/registry.json` 등록 기본값을 라우팅하는지 전수 확인 (status 외 7 skill)

**Checkpoint**: 8 skill 일관 + 가드 작동. SC-003/004/005/007/008 충족.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [ ] T043 hook fail-open 재확인: `cmd_classify_exit` 변경이 PostToolUse exit-0/kill-switch 계약 유지 (CLAUDE.md Hook Safety) + `cargo test hook_safety`
- [ ] T044 게이트 전부: `cargo test` + `bun test`(≥T002 baseline, 0 fail) + `bunx tsc --noEmit` clean
- [ ] T045 [P] tone/keyword lock: `bun run lint:tone --strict`(0 err 해요체) + `bun run lint:keywords --check`(no diff — description byte-lock)
- [ ] T046 [P] (변경 SKILL 있으면) `bun run skill:doctor --strict` exit 0
- [ ] T047 `quickstart.md` §0~§5 실행해 SC-001~008 검증 (live binary 관찰 포함)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup(P1)**: 즉시 시작.
- **Foundational(P2)**: Setup 후. **모든 스토리 차단** (T011 canonical 매핑 잠금 전 편집 금지).
- **US1~US6(P3~8)**: Foundational 후.
- **Polish(P9)**: 원하는 스토리 완료 후.

### User Story Dependencies (공유 파일 현실 반영)

- **US1(P1, MVP)**: Foundational 후 시작. 독립 테스트 가능.
- **US2~US5**: Foundational 후. 독립 *테스트* 가능하나, `data/catalog.json`·`list_deployments.rs`·`error-empathy-catalog.md` 를 **공유 편집**하므로 이 파일 작업은 US1→US2→US3→US4→US5 **순차**. (스토리 병렬 머지는 공유 파일 충돌로 제한.)
- **US6(P1 횡단)**: US1~US5 의 catalog 편집 완료 후 (codegen·guard·sweep 는 최종 상태 필요).

### Within Each User Story

- 테스트(RED) → 구현 → GREEN.
- catalog.json(source) → codegen → generated.md. **각 catalog.json 편집 후 `bun scripts/codegen-catalog.ts` 재실행**해 generated.md 를 동기해요 — 안 그러면 mid-story `bun test` 의 `tests/codegen.test.ts`(hand-written ↔ generated 키 parity)가 fail (CO1). 전체 게이트(T044) 전 반드시 동기 상태. hand-written `error-empathy-catalog.md` 는 각 스토리가 직접 갱신.

### Parallel Opportunities

- Setup: T003 [P].
- Foundational: T004·T005·T006 [P] (서로 다른 확인, data-model 다른 행).
- 스토리 내 [P]: 단위테스트(별 파일) + 서로 다른 skill 파일(T022/T031/T039/T040/T041).
- ❌ 공유 파일(catalog.json/list_deployments.rs/main.rs/catalog.md) 편집은 [P] 아님 — 순차.

---

## Parallel Example: Foundational (Phase 2)

```bash
# T004~T006 동시 (서로 다른 확인 대상):
Task: "auth(4) slug 확정 (error.rs Error::code 전수)"
Task: "rate_limited/tenant/timeout slug 확정"
Task: "apis.call_consent_required 출처 확정"
```

## Parallel Example: US6 skill 정리

```bash
# 서로 다른 skill 파일 → 동시:
Task: "skills/recover/SKILL.md slug 정합 (T039)"
Task: "skills/apps/SKILL.md numeric→slug (T040)"
Task: "skills/init/SKILL.md 65/66→slug/8 (T041)"
```

---

## Implementation Strategy

### MVP First (US1 only)

1. Phase 1 Setup → 2. Phase 2 Foundational (CRITICAL — 매핑·snapshot 잠금) → 3. Phase 3 US1(auth) → 4. **STOP & VALIDATE**: 만료-토큰 재로그인 독립 테스트 → 5. demo.

US1 만으로도 사용자 진입점(status auth)이 고쳐져 가치 전달.

### Incremental Delivery

Foundational → US1(MVP, auth) → US2(not-found) → US3(collision) → US4(rate-limit) → US5(fallback) → US6(single-source+guard). 각 스토리는 catalog 한 조건군을 정합하고 독립 테스트.

### 주의 (공유 파일)

- `data/catalog.json` 은 단일 source — 스토리별 키를 순차 편집, **codegen(T037)은 US6 에서 한 번** (또는 각 스토리 후 재실행해 .md 동기 확인).
- ax-hub-cli 는 **절대 수정 안 함** (Out of Scope) — T003·T047 로 무수정 보증.

---

## Notes

- [P] = 다른 파일 + 미완 의존 없음. 이 기능은 공유 파일 중심이라 [P] 가 적어요 (정직).
- 모든 스토리는 독립 *테스트* 가능; 독립 *병렬 머지* 는 공유 catalog/helper 로 제한.
- T0(Foundational T004~T011) 미완 시 편집 금지 — 잘못된 키 위험.
- 각 task 후 커밋 권장. 게이트(P9)는 머지 전 필수.
- 총 48 tasks (T001~T047 + T042b/FR-008 커버리지).

---

## Implementation Status (spec 004 — 2026-06-02)

### 완료 + 검증 green (cargo lib 248+ · phase_parity · cli_e2e · codegen.test 8 · parity 3 · tone 0 · keywords · doctor · tsc 전부 RC0)

- **Foundational**: slug taxonomy(ErrorCode 11종, auth=`auth`), helper-OUTPUT(EXIT_LIST 65/67) 불변 확정, classify() 라우터 해독, Fork-A, 3-surface 변경집합 (T001/04/05/06/08/10/11).
- **catalog.json ↔ CLI 완전 정합** (parity 가드 green): base `65→4`(auth) · `67→5`(not_found) · `68→6`(rate_limited) · `70→7`(catalog) · `2` 제거(clap 예약). subcode 이름 교정: `scope.downgrade_blocked→update.downgrade_blocked`, `update.cosign_verification_failed→update.cosign_enforce_failed`. not-found subcode 재키 `67:→5:`(github/open/catalog), apis `65:→64:`.
- **S3 classify() subcode-read**: `{exit}:{subcode}→{exit}:{code}→base{exit}` (catalog.rs). non-breaking.
- **helper INPUT** (list_deployments): auth `4|65`+`c=="auth"`, not_found `5|67`+`exit==5` (additive; OUTPUT EXIT_LIST 불변).
- **FR-012 재발방지**: `crates/axhub-helpers/data/cli-exit-contract.json` (pinned) + `tests/exit-contract-parity.test.ts` (3 pass, stale base 색출).
- **skills**: status (auth/notfound/ratelimit→4/5/6 + classify-exit Fork-A), logs (direct routing→4/5/6 + classify-exit), update (cosign subcode명).
- **두 frozen 공간 브리지 (A0-6 — 구현 중 확정)**: classify() 진입부 `normalize_helper_exit`(`65→4/67→5/68→6/70→7`, list_deployments `4|65=>auth` 선례) → CLI-native(status/logs/init/deploy-watch) 와 helper-output(deploy-prep/preflight/list-deployments/token-gate) 둘 다 한 template 으로. catalog 는 CLI-keyed 유지(dual-key 안 함) → parity 가드 그대로 valid. `cli-exit-contract.json` 에 `helper_output_exit_codes`+`helper_output_normalization` 명문화.
- **deploy Step 6 live 버그 수정**: Step5 `deploy status --watch`(CLI 4/5/6) + Step1 `deploy-prep`(helper 65/67) dual-feed 인데 helper만 listing → CLI watch 실패 누락이었음. dual-space(4/65,5/67,6/68) + classify-exit 라우터 포인터로 수정.
- **init CLI-direct 정정**: init 은 `axhub apps bootstrap`/`templates list`(CLI) 호출 → CLI 4/5/6. stale "exit 65(auth)"→`exit 4`/error_code `auth`, forbidden/scope→CLI 12/8. apps/auth catalog 포인터도 CLI "exit 4" 로. `error-empathy-catalog.md` 헤더에 65→4 브리지 note.
- **단위테스트**: classify(4=auth/5=notfound/6=ratelimit/64:subcode) + `helper_output_exits_normalize_into_cli_space`(65/67/68/70 → 동일 template) in catalog.rs; phase_parity/cli_e2e INPUT 신코드 (OUTPUT assert 유지).

### 잔여 (remaining — 모두 진짜 polish/semantic, 라우팅 correctness 는 확보·가드)

- **recover/deploy-prep/token-gate 의 65/67 prose**: helper-output 계약이라 **의도적으로 유지** (정정 대상 아님). classify-exit normalize 가 처리.
- **classify-exit 전면 호출 통일** (남은 helper-mediated skill 의 stylistic Fork-A): prose 가 inline-complete + normalize 가 라우팅하므로 기능 영향 없음 — 순수 consistency polish.
- **profile.endpoint_not_in_allowlist** 의미정제 (현 `66` valid 라 가드 green; `64` usage 계열 검토 — semantic, deferral).
- **공통 fallback 항목** (US5): 미매핑 코드는 classify `default_entry`로 안전 fallback. 전용 항목 추가는 optional polish.

### pre-existing (spec 004 무관 — baseline 확정됨)

- bun **975 pass / 18 fail** — 18 = autowire-statusline(4)/orphan-stub(5)/token-gate subcommand(5+)/schema/README(cross-manifest). 최종 상태에서 fail-set **동일**(US1~A0-6 불변) + 변경영역(catalog/classify/exit-contract/deploy/init/apps/auth) **0개 fail·전부 green** → 회귀 아님, 바이너리 subcommand-presence/worktree env 이슈. (token-gate 테스트가 "exits 65" 단언 = 보존한 helper 계약과 일관.)

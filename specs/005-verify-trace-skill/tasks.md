---
description: "Task list — Trace evidence-source 재설계 (R3γ + R2 + R1)"
---

# Tasks: Trace 스킬 evidence-source 재설계 (skills/trace)

**Input**: `specs/005-verify-trace-skill/` 의 plan.md / spec.md / research.md / data-model.md / contracts/

**Tests**: TDD 요청됨 (research D2–D5 + quickstart DoD + 프로젝트 test-first 표준). 각 스토리에서 테스트를 **먼저 RED** 로 작성 후 구현.

**Organization**: 스토리 라벨 = **보완 항목**(R3/R2/R1), spec.md 의 User Scenarios(US1-3) 와 **별개**예요. **R3**=R3γ 런타임 재설계(P1, MVP) / **R2**=필터 reachability(P2) / **R1**=라벨+docs(P3). spec User Scenarios(vibe coder 추적 등)는 각 스토리의 수용 기준.

## Format: `[ID] [P?] [Story] Description (file path)`

- **[P]**: 다른 파일 · 미완 의존 없음 → 병렬 가능
- **[Story]**: R3/R2/R1 = 보완 항목 (Setup/Foundational/Polish 은 라벨 없음)

## Path Conventions

Rust helper crate + skill: `crates/axhub-helpers/src/`, `crates/axhub-helpers/tests/`, `skills/trace/`, `tests/` (bun). plan.md 구조 기준.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 회귀 감지용 green 기준선 확보

- [x] T001 Green 기준선 캡처 — `cargo test -p axhub-helpers trace` + `bun test tests/trace-skill.test.ts` 실행해 현재 pass 수를 `specs/005-verify-trace-skill/quickstart.md` 노트에 기록

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: R3/R2 가 공유하는 테스트 substrate

**⚠️ CRITICAL**: 이 단계 후 스토리 시작

- [x] T002 [P] NDJSON fixture helper `fake_axhub_app_logs` 추가 (`{"type":"log","message":"..."}` 라인 emit) in `crates/axhub-helpers/tests/cli_e2e.rs`
- [x] T003 [P] 결합 경로(`extract_error_lines`→`match_error_patterns`) 단위 테스트 모듈 scaffold 추가 (현재 :247-275 는 match 직접 호출로 추출 우회) in `crates/axhub-helpers/src/trace_helper.rs`

**Checkpoint**: 공유 fixture 준비 — 스토리 착수 가능

---

## Phase 3: Remediation R3 - R3γ 런타임 로그 evidence 재설계 (Priority: P1) 🎯 MVP

**Goal**: trace 가 `deploy logs` NDJSON 의 `message` 를 파싱해 런타임 실패를 매칭하고, 빌드 단계 실패는 event_log `failure_reason` 으로 안내 — 헤드라인 기능 복구.

**Independent Test**: NDJSON 런타임 로그 fixture 로 `matched_patterns` 발화 + 빈 로그 시 `runtime_log_unavailable` fallback. (수용: spec User Scenario US1 "vibe coder 가 실패 추적" + FR-013)

### Tests for Remediation R3 (RED 먼저) ⚠️

- [x] T004 [P] [R3] Failing e2e: NDJSON 런타임 로그 `message:"env: FOO not found"` → `matched_patterns ⊇ [env_not_found]` in `crates/axhub-helpers/tests/cli_e2e.rs`
- [x] T005 [P] [R3] Failing e2e: 빈 `{"lines":[]}` → `warnings ⊇ [runtime_log_unavailable]` + generic fallback in `crates/axhub-helpers/tests/cli_e2e.rs`
- [x] T006 [P] [R3] Failing unit: `trace()` 가 display-cap 이 아닌 **전체 message** 에 매칭 in `crates/axhub-helpers/src/trace_helper.rs`

### Implementation for Remediation R3

- [x] T007 [R3] `RealTraceProbes::axhub_build_log` NDJSON 파싱 — line 당 `message` 추출, `--source build`/deploy-id 의존 제거, parse 실패 라인 skip + `runtime_log_parse_warning`, 빈 응답 → `runtime_log_unavailable` warning in `crates/axhub-helpers/src/main.rs` (≈2269-2299)
- [x] T008 [R3] 매칭/표시 분리 in `crates/axhub-helpers/src/trace_helper.rs` `trace()` — `matched_patterns` = 전체 message 매칭, `build_log_errors` = ERROR/FATAL/WARN message max 5 (display 계약 불변, NEVER 규칙 보존). `TraceReport` JSON 키 불변
- [x] T009 [R3] 빌드 단계 fallback — `last_phase==Failed` & message 없음 → `event_log.failure_reason` 를 4-part empathy 입력으로 (가능 시 reason 도 `match_error_patterns`) in `crates/axhub-helpers/src/trace_helper.rs`
- [x] T010 [R3] T004/T005/T006 green 확인 + 기존 `cli_trace_json_*` happy-path 회귀 없음 확인 (`cargo test -p axhub-helpers --test cli_e2e`)

**Checkpoint**: trace 가 런타임 로그를 올바르게 소비 — 헤드라인 기능 동작. MVP.

---

## Phase 4: Remediation R2 - R2 reachability + needle 정밀화 (Priority: P2)

**Goal**: 태그 없는 raw 라인의 패턴도 발화(reachability) + substring 오탐 차단. (수용: spec FR-012 + SC-006 reachability)

**Independent Test**: raw 무태그 fixture → 패턴 발화; 오탐 fixture → 미발화. R3 위에서 동작.

### Tests for Remediation R2 (RED 먼저) ⚠️

- [x] T011 [P] [R2] Failing e2e: raw 무태그 `npm ERR! code ELIFECYCLE` (ERROR/WARN 토큰 없음) → `dependency_install_failed` 발화 in `crates/axhub-helpers/tests/cli_e2e.rs`
- [x] T012 [P] [R2] Failing unit: 오탐 guard — `"zoom meeting"` / `"src/room/x"` 가 `oom` **미발화** in `crates/axhub-helpers/src/trace_helper.rs`

### Implementation for Remediation R2

- [x] T013 [R2] `ERROR_PATTERNS` needle 정밀화 in `crates/axhub-helpers/src/trace_helper.rs` — `oom`→`oomkilled`/` oom `/`out of memory` 한정, `exit code 1`→`exit code 127` 접두 오탐 차단(경계 처리)
- [x] T014 [R2] T011/T012 green + R3 패턴 회귀 없음 확인 (`cargo test -p axhub-helpers trace`)

**Checkpoint**: 전체 패턴 도달 가능 + 오탐 없음.

---

## Phase 5: Remediation R1 - R1 라벨 + SKILL/catalog 문구 동기화 (Priority: P3)

**Goal**: SKILL 계약 문구를 런타임 소스로 갱신(D6) + cosmetic 라벨 정리(D7). drift 재발 방지.

**Independent Test**: `skill:doctor`/`lint:tone`/`lint:keywords`/`trace-skill.test.ts` green.

### Tests for Remediation R1 (RED 먼저) ⚠️

- [x] T015 [P] [R1] `tests/trace-skill.test.ts` assertion 갱신 — `event_log + build_log + audit` → runtime-log 문구로 (D6 반영, 먼저 실패시킨 뒤 SKILL 갱신) in `tests/trace-skill.test.ts`

### Implementation for Remediation R1

- [x] T016 [R1] `skills/trace/SKILL.md` 갱신 — 3-source 문구 build_log→runtime_log, Step 2 의 `--source build` 전제 제거, 빌드 단계는 event_log reason 안내 명시 (D6). `description:` trigger 어구는 byte-identical 유지
- [x] T017 [R1] `skills/trace/SKILL.md` D1 guard 의 `trace_target_selection` 라벨을 실제 매칭 기준(registry `trace` 채널 + question text)으로 명확화 (D7/R1) in `skills/trace/SKILL.md`
- [x] T018 [P] [R1] `skills/trace/references/error-patterns.md` 재정렬 — 순수 빌드타임 패턴(dependency_install_failed/docker_image_pull_failed)을 event_log-reason 경로로 라벨 (D6)
- [x] T019 [R1] SKILL 게이트 green 확인 — `bun run skill:doctor --strict` + `bun run lint:tone --strict` + `bun run lint:keywords --check` + `bun test tests/trace-skill.test.ts`

**Checkpoint**: 문구·계약 정합 + 게이트 green.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T020 [P] 전체 Rust 회귀 — `cargo test -p axhub-helpers` + `cargo build --release -p axhub-helpers`
- [x] T021 [P] TS/전체 회귀 — `bunx tsc --noEmit` + `bun test` (≥498 pass / 0 fail 기준선)
- [x] T022 quickstart.md DoD 체크리스트 end-to-end + (선택) 기동된 앱에 `axhub-helpers trace --json` 수동 확인
- [x] T023 `specs/005-verify-trace-skill/spec.md` 검증 갱신 — R3γ/R2/R1 구현 완료 표기, SC-006(reachability) 입증으로 전환, F3 상태 RESOLVED

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (P1)**: 의존 없음
- **Foundational (P2)**: Setup 후 — 모든 스토리 BLOCK
- **R3 (P3)**: Foundational 후. MVP.
- **R2 (P4)**: Foundational 후, **R3 의 message-매칭 위에서 동작** (T008 의존) — 독립 테스트는 가능하나 의미상 R3 먼저
- **R1 (P5)**: Foundational 후. T016 가 D6 를 반영하려면 R3 의 소스 변경 확정 권장 (문구 일치)
- **Polish (P6)**: 모든 스토리 후

### Within Each Story

- 테스트 RED → 구현 → green 확인
- 같은 파일 작업은 순차 (예: T016→T017 둘 다 SKILL.md)

### Parallel Opportunities

- Foundational: T002 ∥ T003 (다른 파일)
- R3 tests: T004 ∥ T005 ∥ T006 (T004/T005 는 cli_e2e.rs 동일 파일 — 실제로는 순차 편집, 논리적 병렬)
- R2 tests: T011 ∥ T012 (다른 파일)
- R1: T018 ∥ T015 (catalog vs bun test, 다른 파일). T016·T017 은 SKILL.md 동일 → 순차
- Polish: T020 ∥ T021

> ⚠️ 같은 파일(`trace_helper.rs`, `cli_e2e.rs`, `SKILL.md`) 다중 작업은 [P] 라벨이어도 실제 편집은 순차 — 충돌 방지.

---

## Parallel Example: Remediation R3

```bash
# R3 구현 핵심(서로 다른 파일):
Task: "RealTraceProbes NDJSON 파싱 in crates/axhub-helpers/src/main.rs"   # T007
# (T008/T009 는 trace_helper.rs 동일 파일 → T007 과는 병렬 가능, 서로는 순차)
```

---

## Implementation Strategy

### MVP First (R3 = R3γ)

1. Setup(T001) → Foundational(T002-T003) → R3(T004-T010)
2. **STOP & VALIDATE**: NDJSON fixture e2e + 빈 로그 fallback green → 헤드라인 기능 복구 확인
3. 여기까지가 최소 의미 있는 fix (trace 가 다시 동작)

### Incremental Delivery

1. R3(R3γ) → 런타임 추적 동작 (MVP)
2. R2(R2) → reachability + 오탐 차단
3. R1(R1) → 문구·계약 정합 + 라벨
4. Polish → 전체 게이트 + spec 갱신

### Notes

- [P] = 다른 파일, 의존 없음. 같은 파일은 순차.
- 테스트 먼저 실패 확인 후 구현.
- 각 task 또는 논리 그룹 후 commit.
- 회피: TraceReport JSON 키 변경, display 5-라인 확대(NEVER 위반), `description:` trigger 변경.
- **out-of-scope (별도 작업)**: deploy hook 의 event_log `failure_reason` 상세화 (research D5) — 빌드 단계 매칭 변별력 향상은 후속.

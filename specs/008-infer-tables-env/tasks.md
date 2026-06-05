---
description: "Task list for infer-tables-env skill"
---

# Tasks: Source-Inferred Tables & Env Recommendations (`infer-tables-env`)

**Input**: Design documents from `specs/008-infer-tables-env/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: 포함함 — 이 프로젝트는 CLAUDE.md authoring gate(`skill:doctor`/`lint:tone`/`lint:keywords`/`bun test`)와 fixture 기반 수용(SC-001/002/005/007)을 필수로 요구해요. 따라서 fixture + 검증 task 를 명시적으로 생성해요.

**Organization**: user story(P1→P2→P3)별 phase. 구현체는 단일 `skills/infer-tables-env/SKILL.md` 라 같은 파일 task 는 병렬 불가 — `[P]` 는 다른 파일(fixtures/registry/baseline/init·deploy)에만 표기해요.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 다른 파일 + 선행 의존 없음 → 병렬 가능
- **[Story]**: US1/US2/US3 (Setup/Foundational/Polish 는 라벨 없음)

## Path Conventions

axhub 플러그인 모노레포: `skills/<name>/SKILL.md`, `tests/fixtures/`, `tests/e2e/`, `.omc/lint-baselines/`. 새 Rust/소스 코드 없음(research D1 — skill-orchestration).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 스캐폴드 + frontmatter + 트리거 + registry 등록

- [X] T001 `bun run skill:new infer-tables-env --model sonnet` 로 스캐폴드 생성 → `skills/infer-tables-env/SKILL.md` (frontmatter `multi-step: true` + `needs-preflight: true` + `model: sonnet`, in-body `CANONICAL_PREFLIGHT_BLOCK`, TodoWrite Step 0, D1 guard, registry stub 자동 삽입)
- [X] T002 `skills/infer-tables-env/SKILL.md` frontmatter `description:` 에 분석형 트리거 어구만 작성 (예: "내 코드 분석해서 테이블/env 추천", "필요한 테이블 뭐야", "필요한 환경변수 추론", "scan my project") — CRUD 어구("테이블 만들"/"env 추가") 금지 (research D6)
- [X] T003 [P] `tests/fixtures/ask-defaults/registry.json` 에 AskUserQuestion 채널(분석/적용 분기) `safe_default: "추천만"` + rationale 등록
- [X] T004 `skills/infer-tables-env/SKILL.md` 트리거 확정 후 `.omc/lint-baselines/skill-keywords.json` 베이스라인 재캡처 (`bun run lint:keywords` regenerate — rare-event, CLAUDE.md 허용)

**Checkpoint**: SKILL 골격 + 트리거 + registry 준비됨

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 story 가 쓰는 공유 오케스트레이션 + 안전 계약. 전부 `skills/infer-tables-env/SKILL.md` 단일 파일 편집 → 순차(비병렬)

**⚠️ CRITICAL**: 이 phase 완료 전 어떤 user story 도 시작 불가

- [X] T005 `skills/infer-tables-env/SKILL.md` Preflight 단계 + `auth_ok`/`auth_error_code` 분기 + 현재 team/app 컨텍스트 확보 + target_app resolve(`axhub apps list`/resolve) 작성 (FR-014)
- [X] T006 `skills/infer-tables-env/SKILL.md` TodoWrite Step 0 체크리스트 + 단계별 status sync + 종료 시 전체 completed 정리 작성
- [X] T007 `skills/infer-tables-env/SKILL.md` `## NEVER` 섹션 작성: 시크릿 값/리터럴 평문 비노출(FR-004/012, SC-005), 승인 없이 mutate 금지(FR-006), env 값 비추론(FR-016), 새 mutation 경로 금지(기존 tables/env 위임)
- [X] T008 `skills/infer-tables-env/SKILL.md` D1 비대화 guard 작성 (`! -t 1` / `$CI` / `$CLAUDE_NON_INTERACTIVE` → AskUserQuestion 건너뛰고 안전 기본값 `추천만`)
- [X] T009 `skills/infer-tables-env/SKILL.md` 로컬-소스-검사 계약 명시 (read-only 파일 검사, governed-data-read 와 반대 성격, 시크릿 비에코 — research D8)

**Checkpoint**: 공유 안전·컨텍스트 계약 완료 — user story 시작 가능

---

## Phase 3: User Story 1 - 코드 분석 → 테이블·env 추천 받기 (Priority: P1) 🎯 MVP

**Goal**: 소스를 분석해서 필요한 테이블(컬럼·타입·제약)과 env 키를 근거와 함께 read-only 로 추천 (mutation 없음)

**Independent Test**: declarative 아티팩트 있는 fixture 앱에 분석 실행 → 기대 테이블·컬럼·env 키가 근거와 함께 표로 나오고, 아무것도 변경 안 됨 확인

### Implementation for User Story 1

- [X] T010 [US1] `skills/infer-tables-env/SKILL.md` 분석 단계 작성: declarative-우선 스캔 순서(schema.prisma/마이그레이션/.env.example → high; ORM 클래스/runtime getenv → low "검토 필요") per `contracts/recommendation-contract.md` C-4, `data-model.md` 매핑
- [X] T011 [US1] `skills/infer-tables-env/SKILL.md` 추천 표 표현 작성: 테이블 표(컬럼·타입·제약·근거·상태) + env 표(키·시크릿?·기본값·근거·상태) + 커버리지 한 줄, per `contracts/recommendation-contract.md` C-1/C-2 (근거 의무 SC-004, 시크릿 비노출 SC-005)
- [X] T012 [US1] `skills/infer-tables-env/SKILL.md` cross-check 작성: `axhub tables list`/`axhub env list` → 상태(신규/이미 있음/검토 필요); target_app 없으면 read-only 추천 + "미확인" 표기(FR-014) per contract C-3
- [X] T013 [US1] `skills/infer-tables-env/SKILL.md` "추론 0건" path(FR-011) + 커버리지 공개(미스캔 영역, FR-013) 작성
- [X] T014 [P] [US1] fixture 생성 `tests/fixtures/infer/nextjs-prisma/` (schema.prisma + .env.example + `expected-recommendation.md` 골든)
- [X] T015 [P] [US1] fixture 생성 `tests/fixtures/infer/fastapi-sqlmodel/` (Alembic 마이그레이션/Settings + .env.example + 골든; 코드-only 모델은 "검토 필요" 기대)
- [X] T016 [P] [US1] secret-safety fixture 생성 `tests/fixtures/infer/secret-hardcoded/` (하드코딩 시크릿 → 보안 발견 플래그, 값 비노출 기대 SC-005)
- [ ] T017 [US1] `tests/e2e/claude-cli/` 매트릭스에 분석 시나리오 추가 (analyze → 추천 표, read-only, 근거 존재 검증)

**Checkpoint**: US1 독립 동작·테스트 가능 — MVP 출하 가능

---

## Phase 4: User Story 2 - 승인하면 바로 적용 (Priority: P2)

**Goal**: 추천 검토 후 승인 → 최종 미리보기(마스킹) → tables/env 스킬로 위임해 테이블 생성 + env 키 등록, 멱등

**Independent Test**: 확정 추천 승인 → 테이블 존재·env 키 등록 확인; 확인 게이트 거절 → 0 변경; 재적용 → 0 변경(멱등)

### Implementation for User Story 2

- [X] T018 [US2] `skills/infer-tables-env/SKILL.md` AskUserQuestion 분기 작성 (`추천만`/`적용`) — registry 채널 일치
- [X] T019 [US2] `skills/infer-tables-env/SKILL.md` 적용 선행 게이트 작성: `auth_ok` + target_app + 최종 마스킹 미리보기 + 명시 확인; 거절 시 0 변경 (FR-006/007/014, SC-006) per `contracts/apply-handoff-contract.md` C-1
- [X] T020 [US2] `skills/infer-tables-env/SKILL.md` 테이블 생성 위임 작성: consent-mint `table_create`/`table_alter` → `axhub tables create`/`add-column`, status=new 만(멱등), per apply-handoff C-2/C-5 (FR-008/009)
- [X] T021 [US2] `skills/infer-tables-env/SKILL.md` env 등록 위임 작성: consent-mint `env_set` → `printf %s | axhub env set --from-stdin`, 키만 등록·값은 stdin/건너뜀·비시크릿 기본값 prefill, per apply-handoff C-3 (FR-016)
- [X] T022 [US2] `skills/infer-tables-env/SKILL.md` 항목별 결과 보고 작성 (success/failed/skipped, FR-008)
- [X] T023 [P] [US2] 멱등 fixture 생성 `tests/fixtures/infer/already-configured/` (재적용 → 0 변경 기대, SC-007)
- [ ] T024 [US2] `tests/e2e/claude-cli/` 매트릭스에 적용 시나리오 추가 (approve → preview → consent → create/register; deny → no change)

**Checkpoint**: US1 + US2 둘 다 독립 동작

---

## Phase 5: User Story 3 - 개발 흐름 중 자동 제안 (Priority: P3)

**Goal**: init/deploy 같은 라이프사이클 순간에 경량 비차단 넛지, 수락 시에만 전체 분석

**Independent Test**: 라이프사이클 순간 발생 → 넛지 표시(전체 스캔 안 함); 거절 → 부작용 0

### Implementation for User Story 3

- [X] T025 [US3] `skills/infer-tables-env/SKILL.md` 경량 넛지 지침 작성 (넛지 시점엔 전체 스캔 금지, 수락 후에만 전체 분석; FR-010) per research D5
- [X] T026 [P] [US3] `skills/init/SKILL.md` 흐름 끝에 한 줄 넛지 추가 ("필요한 테이블·환경변수 추천해드릴까요?", 비차단)
- [X] T027 [P] [US3] `skills/deploy/SKILL.md` 배포 **직전(선행)** 지점에 한 줄 넛지 추가 ("필요한 테이블·환경변수 추천해드릴까요?", 비차단·거절 무부작용 — spec US3 "배포 직전" 타이밍 일치)

**Checkpoint**: 세 story 모두 독립 동작

---

## Phase 6: Polish & Cross-Cutting Concerns (Authoring Gates)

**Purpose**: CLAUDE.md authoring gate + 문서 검증

- [X] T028 `bun run skill:doctor --strict` 실행 → D1/TodoWrite/in-body preflight/step-collision 통과 (`skills/infer-tables-env/SKILL.md`)
- [X] T029 `bun run lint:tone --strict` → 해요체 0 err
- [X] T030 `bun run lint:keywords --check` → 베이스라인 drift 없음 확인
- [X] T031 `bun test` → ux-* 패턴 회귀 + `tests/ux-ask-fallback-registry.test.ts` 통과
- [X] T032 `bunx tsc --noEmit` → clean
- [ ] T033 [P] `CHANGELOG.md` 에 새 SKILL `infer-tables-env` 항목 추가 + `docs/plugin-developer-guide.md` skill 목록에 한 줄 반영(해당 시)
- [ ] T034 `quickstart.md` end-to-end 검증 (fixture 앱 분석→승인→적용 1회 수동 통과; 코드→설정 소요 시간 측정으로 SC-003 확인)
- [ ] T035 [P] FR-015 검증: 분석/적용 후 추천 결과가 어떤 파일에도 저장되지 않음 확인 — `tests/e2e/claude-cli/` 시나리오에 "추천 직후 작업트리 변화 없음(휘발성)" assert 추가

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (P1)**: 의존 없음. T002→T004(트리거 확정 후 베이스라인). T003 병렬.
- **Foundational (P2)**: Setup 후. T005~T009 전부 같은 SKILL.md → **순차**. 모든 user story 차단.
- **User Stories (P3+)**: Foundational 후. US1→US2 는 SKILL.md 본문 공유라 사실상 순차 권장(독립 테스트는 가능). US3 는 init/deploy 파일이라 일부 병렬.
- **Polish (P6)**: 원하는 story 완료 후. 게이트는 마지막에 일괄.

### User Story Dependencies

- **US1 (P1)**: Foundational 후 시작. 다른 story 의존 없음 (MVP).
- **US2 (P2)**: Foundational 후. US1 추천 산출에 논리 의존하나 적용 자체는 독립 테스트 가능.
- **US3 (P3)**: Foundational 후. US1/US2 와 독립 (넛지만).

### Within Each Story

- 같은 `SKILL.md` 편집 task 는 순차 (파일 충돌 회피).
- fixture/e2e task 는 본문 task 와 병렬 가능([P]).

### Parallel Opportunities

- T003 (registry) ∥ T002 (frontmatter)
- T014/T015/T016 (fixtures, 서로 다른 디렉토리) 동시
- T023 (멱등 fixture) ∥ US2 본문 task
- T026/T027 (init/deploy 넛지, 다른 파일) 동시
- ⚠️ T005~T013, T018~T022, T025 는 모두 `skills/infer-tables-env/SKILL.md` → 병렬 불가

---

## Parallel Example: User Story 1

```bash
# US1 fixtures 동시 생성 (서로 다른 디렉토리):
Task: "tests/fixtures/infer/nextjs-prisma/ 생성 (schema.prisma + .env.example + 골든)"
Task: "tests/fixtures/infer/fastapi-sqlmodel/ 생성 (마이그레이션/Settings + 골든)"
Task: "tests/fixtures/infer/secret-hardcoded/ 생성 (시크릿 비노출 기대)"
# SKILL.md 본문(T010~T013)은 같은 파일이라 순차로 진행
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 Setup (스캐폴드·트리거·registry)
2. Phase 2 Foundational (preflight·NEVER·D1·로컬검사 계약)
3. Phase 3 US1 (분석 + 추천 표, read-only)
4. **STOP & VALIDATE**: fixture 로 US1 독립 검증 (근거·시크릿 비노출·0 변경)
5. 게이트(T028~T032) 통과하면 MVP 데모 가능

### Incremental Delivery

1. Setup + Foundational → 기반 준비
2. US1 → 독립 검증 → 데모 (MVP: read-only 추천)
3. US2 → 독립 검증 → 데모 (승인 시 적용)
4. US3 → 독립 검증 → 데모 (자동 넛지)
5. Polish 게이트 일괄 → ship

---

## Notes

- 단일 SKILL.md 구현체라 [P] 는 fixtures/registry/baseline/init·deploy 등 **다른 파일** task 에만.
- 새 Rust/소스 코드 없음 — 기존 `tables`/`env` 스킬 + `axhub-helpers preflight/consent-mint` + `axhub` CLI 위임 (research D1).
- recall/precision(SC-001/002)은 fixture 골든 평가(결정론 유닛테스트 아님 — research D1 트레이드오프).
- 시크릿 안전(SC-005)은 SKILL `NEVER` + env 마스킹 + 승인 게이트로 확보(코드 강제 아님).
- 각 task 후 또는 논리 그룹마다 커밋. checkpoint 에서 story 독립 검증.
- T026/T027 은 `init`/`deploy` SKILL 을 편집하므로(cross-skill), 그 변경분도 T029(`lint:tone`)·T030(`lint:keywords`)·T031(`bun test`) repo-wide 게이트 대상에 포함돼요.

## Deferred (staging/live/release 필요 — 이 컨텍스트서 검증 불가)

- T017 / T024 / T035 — `tests/e2e/claude-cli/` 매트릭스 배선은 `claude -p` + mock-hub staging 환경 필요. 기대 시나리오는 `tests/fixtures/infer/*/expected-recommendation.md` 골든으로 작성 완료(분석·시크릿비노출·멱등). staging 가용 시 case.sh 로 이식.
- T033 — CHANGELOG 는 이 repo 의 release-flow(commit-and-tag-version + narrative)가 자동 생성. `feat:` 커밋이 릴리스 때 항목 생성하므로 수동 편집 안 함. docs/plugin-developer-guide.md 는 per-skill 목록이 없어 반영 대상 아님(N/A). README skill 카운트(43→44)는 반영 완료.
- T034 — quickstart end-to-end(분석→승인→적용)는 live 인증 + 실제 app + 배포 필요라 로컬서 실행 불가. read-only 분석 경로는 fixtures 로 검증 가능.

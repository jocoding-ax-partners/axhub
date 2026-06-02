---
description: "Task list — update 스킬 ↔ ax-hub-cli v0.17.2 계약 정렬"
---

# Tasks: update 스킬 ↔ ax-hub-cli v0.17.2 계약 정렬

**Input**: `specs/005-update-skill-cli-alignment/` 설계 문서
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/update-cli-contract.md, quickstart.md

**Tests**: 신규 테스트 **없음**. spec 이 새 test 를 요청하지 않았고 drift-guard 는 범위 밖(spec Clarifications)이에요. 검증은 기존 게이트(`bun test`/`skill:doctor`/lint) + grep + live `axhub update --help` 로 해요.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 가능 (다른 파일, 미완 의존 없음)
- **[Story]**: US1/US2/US3 (spec.md user story)
- ⚠️ **단일 파일 제약**: US1/US2/US3 의 `skills/update/SKILL.md` 편집은 **같은 파일**이라 서로 [P] 불가 — 순차. catalog 파일(`catalog.json`/`*.generated.md`/`error-empathy-catalog.md`)은 별개라 SKILL 편집과 [P].

---

## Phase 1: Setup

- [x] T001 live CLI surface 를 contract §1 과 대조 — `~/.axhub/bin/axhub update --help` + `~/.axhub/bin/axhub update apply --help` 실행, 플래그(`--dry-run`/`--execute`/`--yes`/`--force`)·subcommand 일치 확인, delta 있으면 `specs/005-update-skill-cli-alignment/research.md` 에 기록
- [x] T002 [P] 변경 전 게이트 baseline green 캡처 (회귀 귀속용) — `bun test` + `bun run skill:doctor --strict` + `bunx tsc --noEmit` 실행해 현 통과 상태 기록
- [x] T003 [P] 편집 사이트 열거 — `rg -n "AXHUB_REQUIRE_COSIGN|AXHUB_ALLOW_UNSIGNED|AXHUB_DISABLE_AUTOUPDATE|package_manager|brew|scoop|cosign_verification_failed|exit 2" skills/update/SKILL.md` 결과 줄번호를 작업 체크리스트로 정리

## Phase 2: Foundational (모든 story 의 blocking 선행)

- [x] T004 catalog 키 blast radius 결정 (research D10) — `rg -n "cosign_verification_failed" crates/ skills/ scripts/ tests/` + helper/스킬이 CLI `error.subcode` 로 catalog 를 lookup 하는지 확인. 결론: catalog.json 키 rename 을 US2 범위에 포함할지 vs SKILL.md 한정으로 trim 할지 결정 (T014/T015 를 gate)
- [x] T005 frontmatter `description:` byte-lock 경계 확정 — `bun run lint:keywords --check` 가 편집 전 clean 임을 확인(이후 diff 는 description 침범 신호). `skills/update/SKILL.md` frontmatter 는 **수정 금지 영역**으로 표시

**Checkpoint**: T004 결정 + T005 경계 확정 후 story 진입.

---

## Phase 3: User Story 1 — 업그레이드가 실제 CLI 에서 그대로 동작 (P1) 🎯 MVP

**Goal**: 사용자가 업그레이드 동의 시, 스킬이 v0.17.2 가 수용하는 명령만 실행하고(가공 env 없음) cosign 검증 유지된 채 성공.

**Independent Test**: `~/.axhub/bin/axhub update apply --dry-run --json` 으로 preview 구동 → 스킬 문서화 명령이 전부 수용(unexpected-arg 없음), execute 호출에 가공 env 접두 없음, cosign 기본 enforce.

- [x] T006 [US1] `skills/update/SKILL.md` Step 4 execute 호출에서 `AXHUB_REQUIRE_COSIGN=1` 접두 제거 → `axhub update apply --execute --yes --json` (기본 enforce cosign 의존, contract §4)
- [x] T007 [US1] `skills/update/SKILL.md` Step 4 의 `axhub update apply --dry-run --json` preview 유지하고, contract §2 의 preview 필드(`current`/`latest`/`is_downgrade`/`feed_base`/`next_step`)를 동의 전 사용자에게 노출
- [x] T008 [US1] `skills/update/SKILL.md` Step 4 성공 처리를 `{applied:true, install_kind, current, latest, binary}` (contract §2) 파싱으로 갱신, 해요체 완료 카드("새 터미널 / `axhub --version`") 렌더
- [x] T009 [US1] 검증 — `~/.axhub/bin/axhub update apply --help` 플래그 확인 + `axhub update apply --dry-run --json` 수용 확인, 스킬 apply 명령 ⊆ contract §1

**Checkpoint**: US1 단독으로 정상 apply 경로 동작 — MVP 출하 가능.

---

## Phase 4: User Story 2 — 종료 코드·결과를 정확히 해석 (P2)

**Goal**: exit 14/15/66 + `error.subcode` 를 contract §3 / docs/cli-exit-codes.md 대로 매핑.

**Independent Test**: update 관련 각 종료 코드(0/1/4/10/14/15/64/66)에 대해 스킬 문서화 대응이 contract §3 규정 행동과 1:1 일치.

- [x] T010 [US2] `skills/update/SKILL.md` exit 라우팅 정렬 — (a) cosign subcode `update.cosign_verification_failed` → `update.cosign_enforce_failed` (약 line 115,152), exit 66 cosign 하드 스톱·우회 없음 유지; (b) Step 7 "other non-zero" 의 **stale `65/68/1` → 실제 코드(1/4/10/64)로 교체** (exit 65/68 은 계약에 없음); (c) **exit 4 미인증 → `axhub auth login` 유도** 추가; 1/10/64 는 error-empathy-catalog 라우팅 보존
- [x] T011 [US2] `skills/update/SKILL.md` 에 exit 14 (VerifyDigestMismatch) 처리 추가 — 변조 신호 즉시 중단, `--force` 금지, IT/보안 통보 (data-model 상태전이)
- [x] T012 [US2] `skills/update/SKILL.md` 에 exit 15 (SwapFailed) 처리 추가 — 자동 재시도 금지(부분 교체), `~/.axhub/bin/axhub.<old>.bak` 롤백 안내
- [x] T013 [US2] `skills/update/SKILL.md` 에 exit 66 + `update.downgrade_blocked` 처리 추가 — `--force` 를 cosign 안전 downgrade 우회로 안내(cosign-enforce 와 구분)
- [ ] T014 [P] [US2] ⏸️ **DEFERRED (T004=trim: blast radius)** — `crates/axhub-helpers/data/catalog.json` 키 `update.cosign_verification_failed` → `update.cosign_enforce_failed`, `bun run codegen:catalog` regen. **보류 사유**: catalog.json 키 변경이 Rust 테스트 `crates/axhub-helpers/tests/phase_parity.rs:214` (stale code 단언) 동반 수정 + codegen regen 을 강제 → "skill doc 정렬" 범위 초과. follow-up 으로 분리 (cargo test 검증 동반 별도 PR).
- [ ] T015 [P] [US2] ⏸️ **DEFERRED (T014 와 동반)** — hand-authored `skills/deploy/references/error-empathy-catalog.md` (line 160) 헤더를 `update.cosign_enforce_failed` 로 정정. SKILL.md 는 실제 CLI subcode 로 이미 맞췄고 catalog 라우팅은 "cosign 항목" semantic 이라 동작 — catalog heading 정정은 follow-up.
- [x] T016 [US2] 검증 — `bun test tests/codegen.test.ts` (catalog.json↔generated 일치) + contract §3 각 행(0/1/4/10/14/15/64/66) ↔ `skills/update/SKILL.md` 교차 대조로 각 1개 대응 확인

**Checkpoint**: US2 단독으로 모든 실패 경로 해석 정확.

---

## Phase 5: User Story 3 — 존재하지 않는 명령·env·우회 제거 (P3)

**Goal**: CLI 미지원 env/flag/분기를 본문에서 완전 제거.

**Independent Test**: 본문 grep 으로 가공 env/brew = 0, `--force` 설명이 `apply --help` 의미와 일치.

- [x] T017 [US3] `skills/update/SKILL.md` Step 1 + NEVER 에서 `AXHUB_ALLOW_UNSIGNED` 언급 + exit-2/`AXHUB_DISABLE_AUTOUPDATE` "회사 정책 disable" 시나리오 제거 (AXHUB_REQUIRE_COSIGN 은 T006 에서 이미 제거)
- [x] T018 [US3] `skills/update/SKILL.md` 의 brew/scoop 감지 분기(Step 6 `package_manager:"brew"`) 통째 제거, generic 재설치 note 도 안 둠 (spec Clarifications)
- [x] T019 [US3] `skills/update/SKILL.md` NEVER 절을 정정 — unsigned-bypass 문구 삭제, "`--force` 는 cosign 우회 아님" + "exit 14 / cosign-66 절대 우회 금지" 로 교체
- [x] T020 [US3] 검증 — `rg "AXHUB_REQUIRE_COSIGN|AXHUB_ALLOW_UNSIGNED|AXHUB_DISABLE_AUTOUPDATE" skills/update/SKILL.md` = 0, `rg "package_manager|brew|scoop" skills/update/SKILL.md` = 0, `--force` 설명 ↔ `apply --help` 일치

**Checkpoint**: US3 단독으로 죽은 지시 0.

---

## Phase 6: Polish & Cross-Cutting

- [x] T021 [P] `bun run lint:keywords --check` no diff (frontmatter `description:` byte-lock 보존 확인)
- [x] T022 [P] `bun run lint:tone --strict` 0 err (본문 해요체)
- [x] T023 `bun run skill:doctor --strict` exit 0 (D1 guard / TodoWrite Step 0 / frontmatter 선언 패턴 보존)
- [x] T024 `bun test` (ux-todowrite / ux-ask-fallback-registry / codegen 등 회귀 전부 pass) + `bunx tsc --noEmit` clean
- [x] T025 최종 일관성 — `skills/update/SKILL.md` 전체 재독해 vs contract, TodoWrite Step 0 + D1 비대화형 guard 무손상, orphan 참조 0, spec SC-001..005 충족 확인. live `axhub update apply --dry-run --json` 1회 spot-check

---

## Dependencies & 실행 순서

```
Phase 1 (Setup: T001-T003)
   ↓
Phase 2 (Foundational: T004 blast-radius 결정, T005 byte-lock 경계) ← BLOCKS 모든 story
   ↓
Phase 3 US1 (T006-T009)  ─┐  SKILL.md 같은 파일 → story간 순차
Phase 4 US2 (T010-T016)  ─┤  (단 T014/T015 catalog 파일은 SKILL 편집과 [P])
Phase 5 US3 (T017-T020)  ─┘
   ↓
Phase 6 Polish (T021-T025) ← 모든 story 후
```

- **T004 gate**: T014/T015(catalog) 는 T004 가 "in-scope" 결정해야 진행. "trim" 결정 시 T014/T015 skip 하고 catalog 정정을 별도 follow-up 으로.
- **단일 파일**: T006-T008, T010-T013, T017-T019 모두 `skills/update/SKILL.md` → 순차 편집(같은 파일 [P] 금지). 논리적으로 story별 독립 슬라이스라 순서는 P1→P2→P3 권장.
- **story 독립성**: 각 story 의 검증(T009/T016/T020)은 단독 실행 가능.
- **⚠️ P1 (branch 명명 caveat)**: 현 branch `worktree-mighty-squishing-finch` 는 speckit `NNN-name` 규칙 미충족이라 `check-prerequisites.sh --require-tasks` 가 abort 해요(파일은 다 존재, feature.json 으로 specify/plan/tasks 는 동작). `/speckit-implement` 가 같은 abort 시: (a) `git branch -m worktree-mighty-squishing-finch 005-update-skill-cli-alignment` 로 rename 후 진행, 또는 (b) 이 tasks.md 를 수동으로 따라가요. worktree 디렉터리명은 그대로라 harness cleanup 영향 없어요.

## 병렬 기회

- Phase 1: T002 + T003 [P] (read-only 측정, 서로 무관)
- Phase 4: T014 + T015 [P] (catalog 파일들, SKILL.md 편집 T010-T013 과도 다른 파일이라 동시 가능)
- Phase 6: T021 + T022 [P] (독립 lint)

## Implementation Strategy

- **MVP = US1 (P1)**: 가공 env 없이 apply 경로가 실제 CLI 에서 동작 — 스킬 존재 이유의 핵심. 여기까지만 해도 "업데이트해줘" 가 깨지지 않아요.
- **증분**: US2(실패 경로 안전) → US3(죽은 지시 청소) 순. 각 story 후 해당 검증 task 로 독립 확인.
- **안전**: cosign/변조 관련(US2)은 보안 critical — T010-T013 후 contract §3 대조를 반드시 통과.
- **scope 가드**: catalog blast radius(T004)가 과하면 US2 의 catalog 부분(T014/T015)을 잘라내고 SKILL.md 정렬만 출하, catalog 는 follow-up.

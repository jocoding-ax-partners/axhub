---
description: "Task list — verify 스킬 ↔ ax-hub-cli v0.17.2 + axhub-helpers 계약 정렬"
---

# Tasks: verify 스킬 ↔ ax-hub-cli v0.17.2 + axhub-helpers 계약 정렬

**Input**: `specs/006-verify-skill-cli-alignment/` 설계 문서
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/verify-cli-contract.md, quickstart.md

**Tests**: 신규 테스트 **없음**. spec 이 새 test 를 요청 안 했고 drift-guard 는 별도 feature(Clarifications). 검증 = 기존 게이트(`bun test`/`skill:doctor`/lint/`cargo test`) + grep + live `--help`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 가능 (다른 파일, 미완 의존 없음)
- ⚠️ **단일 파일 제약**: US1/US2/US3 의 `skills/verify/SKILL.md` 편집은 **같은 파일**이라 서로 [P] 불가 — 순차. (005 와 달리 catalog/Rust/registry 변경 없음 — recover SKILL 은 read-only 참조.)

---

## Phase 1: Setup

- [x] T001 live CLI/helper surface 를 contract §1-5 와 대조 — `~/.axhub/bin/axhub deploy {status,list,logs} --help` + `axhub-helpers verify --help`(또는 `verify_helper.rs` 소스). delta 있으면 `specs/006-verify-skill-cli-alignment/research.md` 에 기록
- [x] T002 [P] 변경 전 게이트 baseline green 캡처 — `bun test` + `bun run skill:doctor --strict` + `bunx tsc --noEmit` + `cargo test -p axhub-helpers` 통과 상태 기록 (회귀 귀속용)
- [x] T003 [P] 편집 사이트 열거 — `rg -n "verdict.*passed|--source pod|deploy logs <DEPLOY|app-id 도 alias|active / succeeded / live" skills/verify/SKILL.md` 줄번호 정리

## Phase 2: Foundational (모든 story blocking 선행)

- [x] T004 status/`--source` live audit 확정 (research D2/D3) — `axhub deploy logs --app <app> --json | head` 로 app-level 로그 수신 확인(deployment_id/`--source pod` 불요), `axhub deploy status <id> --app --json` 의 `.status` 가 free string 임을 확인. live-state 기준 = helper `LIVE_STATES`(`live/running/deployed/active/ok/succeeded`)로 고정
- [x] T005 보존 경계 확정 — `bun run lint:keywords --check` clean 확인(이후 diff=description 침범), `skills/verify/SKILL.md` 의 frontmatter `description:` + in-body CANONICAL_PREFLIGHT_BLOCK + D1 가드 + TodoWrite Step 0 + health AskUserQuestion 을 **수정 금지 영역**으로 표시

**Checkpoint**: T004 audit 확정 + T005 경계 후 story 진입.

---

## Phase 3: User Story 1 — verdict 가 실제 helper 출력과 일치 (P1) 🎯 MVP

**Goal**: 스킬이 문서화·파싱하는 `axhub-helpers verify` JSON 이 실제 `VerifyResult` 와 일치, verdict 매핑 정확.

**Independent Test**: 실제 `axhub-helpers verify --json --app-id <app>` 스키마(verdict∈{live,suspect,not_live}, state, last_deploy_id, last_deploy_age_secs, errors, reasons)와 SKILL 문서 대조.

- [x] T006 [US1] `skills/verify/SKILL.md` CI 예시(약 line 119-120) `{"verdict":"passed"}` → 실제 `VerifyResult` 모양(`{"verdict":"live","state":"active","last_deploy_id":"...","last_deploy_age_secs":120,"errors":[],"reasons":["..."]}`) + 명령 `axhub-helpers verify --json --app-id <app>`
- [x] T007 [US1] `skills/verify/SKILL.md` verdict 매핑 — helper `verdict` ∈ {live,suspect,not_live} → ✅ 라이브 / ⚠️ 의심 / ❌ 안 됨, `reasons` 배열을 verdict 아래 verbatim 출력
- [x] T008 [US1] 검증 — `axhub-helpers verify --json` 출력 스키마(`verify_helper.rs` VerifyResult) ↔ SKILL 문서 1:1, `verdict:"passed"` 0건

**Checkpoint**: US1 단독으로 verdict 정확.

---

## Phase 4: User Story 2 — 모든 명령·플래그가 실제 CLI 에서 수용 (P2)

**Goal**: deploy status/list/logs + helper verify/list-deployments 명령·플래그 + status 분기가 실제 계약과 일치.

**Independent Test**: 각 명령 live `--help` 수용("unknown command/argument" 0건), live-state 판정이 LIVE_STATES 부분집합.

- [x] T009 [US2] `skills/verify/SKILL.md` Step 2 — `axhub deploy status <id> --app <app> --json` 유지, **status 를 닫힌 enum 으로 서술 제거** → LIVE_STATES(`live/running/deployed/active/ok/succeeded`, `ok` 포함) 휴리스틱 + 그 외=미라이브(진행중/실패는 "예시 휴리스틱"으로 명시), `.current_stage` 단계 안내
- [x] T010 [US2] `skills/verify/SKILL.md` Step 3 — **app-level** `axhub deploy logs --app <app> --json` 로 교체 (`<DEPLOY_ID>` 스코핑 + `--source pod` 제거; deployment_id legacy, source 고정 enum 없음). client-side 마지막 ~50줄 trim → ERROR/FATAL grep 유지
- [x] T011 [US2] `skills/verify/SKILL.md` Step 1 — helper `list-deployments --app-id <app> --limit 1` (primary 인자명), `deploy list --app <app> --json` (no `--limit`) 확인
- [x] T012 [US2] 검증 — `axhub deploy {status,list,logs} --help` + `axhub-helpers verify --help` 로 모든 명령·플래그 수용 확인, status 판정 ↔ LIVE_STATES 일치

**Checkpoint**: US2 단독으로 명령 전부 수용.

---

## Phase 5: User Story 3 — 오해 소지 문서·죽은 가정 제거 (P3)

**Goal**: 실제 계약과 어긋난 설명 제거.

**Independent Test**: 본문 grep — `verdict.*passed`=0 / `--source pod`=0 / `<DEPLOY_ID>` logs 스코핑=0, `--app-id`/`--app` 설명 정확.

- [x] T013 [US3] `skills/verify/SKILL.md` — `--app-id`(primary)/`--app`(alias) 설명 정정 (스킬의 "--app-id 도 alias" 반대로 수정)
- [x] T014 [US3] `skills/verify/SKILL.md` — live-state 목록에 `ok` 포함 + `.status` 가 백엔드 free string 임을 명시(닫힌 enum 가정 제거)
- [x] T015 [US3] `skills/verify/SKILL.md` — helper error_code 분기를 `../recover/SKILL.md` 정본 표 cross-link 으로 유지 확인 (정정 불요, 표 복제 금지)
- [x] T016 [US3] 검증 — `rg "verdict.*passed|--source pod|deploy logs <DEPLOY" skills/verify/SKILL.md` = 0, `--app-id`/`--app` 설명 ↔ `cli/args/mod.rs` 일치

**Checkpoint**: US3 단독으로 죽은 가정 0.

---

## Phase 6: Polish & Cross-Cutting

- [x] T017 [P] `bun run lint:keywords --check` no diff (frontmatter `description:` byte-lock 보존)
- [x] T018 [P] `bun run lint:tone --strict` 0 err (본문 해요체)
- [x] T019 `bun run skill:doctor --strict` exit 0 (in-body preflight 블록 / D1 가드 / TodoWrite Step 0 / needs-preflight 계약 보존)
- [x] T020 `bun test` (ux-todowrite / ux-ask-fallback-registry 등) + `bunx tsc --noEmit` + `cargo test -p axhub-helpers` (verify_helper 무변경 회귀 green 유지)
- [x] T021 최종 일관성 — `skills/verify/SKILL.md` 전체 재독 vs contract, in-body preflight 무손상, health AskUserQuestion/registry 유지, orphan 참조 0, spec SC-001..005 충족. live `axhub deploy logs --app <app> --json` 1회 spot-check

---

## Dependencies & 실행 순서

```
Phase 1 (Setup: T001-T003)
   ↓
Phase 2 (Foundational: T004 audit 확정, T005 보존 경계) ← BLOCKS 모든 story
   ↓
Phase 3 US1 (T006-T008)  ─┐  SKILL.md 같은 파일 → story간 순차
Phase 4 US2 (T009-T012)  ─┤  (catalog/Rust/registry 변경 없음 — recover read-only)
Phase 5 US3 (T013-T016)  ─┘
   ↓
Phase 6 Polish (T017-T021) ← 모든 story 후
```

- **단일 파일**: T006-T007, T009-T011, T013-T015 모두 `skills/verify/SKILL.md` → 순차(같은 파일 [P] 금지). 순서 P1→P2→P3 권장.
- **read-only**: `skills/recover/SKILL.md`(error_code 정본), `crates/axhub-helpers/src/verify_helper.rs`(VerifyResult) 는 참조만 — 편집 0.
- **story 독립성**: 각 검증(T008/T012/T016) 단독 실행 가능.
- **⚠️ P1 (branch/PR caveat)**: 현 branch `feat/update-skill-cli-alignment` 는 speckit `NNN-name` 미충족이라 `check-prerequisites.sh --require-tasks` 가 abort 해요(파일 존재, feature.json 으로 동작). `/speckit-implement` abort 시: rename 또는 수동 진행. 또 006(verify) spec 이 005(update) PR #155 브랜치에 함께 있어서, verify 구현은 **별도 PR/브랜치로 분리 권장** (update 와 성격 분리).

## 병렬 기회
- Phase 1: T002 + T003 [P]
- Phase 6: T017 + T018 [P]

## Implementation Strategy
- **MVP = US1 (P1)**: verdict 정확성(`passed`→`live`) — verify 의 핵심 산출물. JSON 소비자(CI)+스킬 둘 다 올바른 verdict.
- **증분**: US2(명령 수용/app-level logs/status 휴리스틱) → US3(문서 청소). 각 story 후 검증.
- **단순성**: 005(update)와 달리 catalog codegen·Rust·registry 변경 없음. audit 에서 CLI 버그 없음 확인 → skill-only. recover 표는 read-only.
- **scope 가드**: 재-drift guard 는 이 feature 밖(별도 공용 feature). status/`--source` 가 만약 CLI 버그로 판명되면 verify 는 현 CLI 에 맞추고 CLI fix 는 별도 cross-repo.

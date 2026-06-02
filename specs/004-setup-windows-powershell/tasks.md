---
description: "Task list — setup 스킬 Windows PowerShell 지원 + CLI v0.17.2 정합"
---

# Tasks: setup 스킬 Windows PowerShell 지원 + CLI v0.17.2 정합

**Input**: `specs/004-setup-windows-powershell/` design 문서
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/setup-command-matrix.md

**Tests**: TDD 미요청. 단일 SKILL.md 문서 리팩토링이라 unit test 부적합 — 검증은 정적 게이트(skill:doctor/lint/test/tsc) + 커버리지 grep + 행위 검증(quickstart)으로 Polish phase 에서 수행해요.

**Organization**: user story 별 그룹. 단 모든 구현이 단일 `skills/setup/SKILL.md` 편집이라 같은 파일 task 는 순차([P] 없음)예요.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 가능 (다른 파일, 의존 없음)
- **[Story]**: US1/US2/US3 (spec.md 매핑)

## Path Conventions

- 수정 대상: `skills/setup/SKILL.md` (단일 파일 in-place)
- 차용 참조(읽기 전용): `skills/{install-cli,doctor,upgrade}/SKILL.md`, `skills/deploy/references/recovery-flows.md`

---

## Phase 1: Setup

**Purpose**: 차용할 cross-platform 패턴 레퍼런스 확정 (research Decision 3)

- [X] T001 [P] (supports US1 — FR-001~005 선행 레퍼런스) `$env:OS` 3분기(install-cli), `.exe` helper-pick + cache-scan(doctor), `$env:USERPROFILE` 경로(recovery-flows), ConvertFrom-Json + `Unix / Git Bash:`/`Windows PowerShell:` 라벨(upgrade PR#160) 패턴을 skills/install-cli/SKILL.md · skills/doctor/SKILL.md · skills/deploy/references/recovery-flows.md · skills/upgrade/SKILL.md 에서 확인

---

## Phase 2: Foundational

**해당 없음** — 단일 `skills/setup/SKILL.md` in-place 리팩토링이라 US 전 차단(blocking) 작업이 없어요. 위임 모델상 sibling 스킬·crate·스키마 변경 없음.

---

## Phase 3: User Story 1 — Windows PowerShell 첫 사용자 온보딩 (Priority: P1) 🎯 MVP

**Goal**: setup 의 OS 의존 셸 명령에 Windows PowerShell 등가를 추가해 PowerShell 세션에서도 온보딩 완주.

**Independent Test**: Windows PowerShell 에서 Step 1 감지 → 카드 → gap 위임 → Step 4 node → 준비 카드가 bash 없이 동작 (spec US1-AC1~3).

- [X] T002 [US1] skills/setup/SKILL.md Step 1 상태 감지 블록에 `Windows PowerShell:` 등가 추가 — `axhub --version`/`node --version`(동일), lockfile `Get-ChildItem`, `.nvmrc` `Get-Content`, engines 읽기. 기존 bash 위에 `Unix / Git Bash:` 라벨도 추가 (contracts Step 1 행, FR-001)
- [X] T003 [US1] skills/setup/SKILL.md Step 1 helper preflight 블록에 `Windows PowerShell:` 등가 추가 — `& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" preflight --json` + cache-scan 폴백(`$env:USERPROFILE\.claude\plugins\cache\axhub\axhub\*\bin\axhub-helpers.exe`). doctor 단순도 — awk/sort 등가물 금지 (FR-002, research Decision 3)
- [X] T004 [US1] skills/setup/SKILL.md Step 4 node 설치에 Windows 경로 명문화 — winget → scoop → nodejs.org LTS 수동 안내. unix `nvm curl|bash` 자동 2순위는 Windows 미적용(플랫폼 제약). consent-gate 유지 (FR-003, research Decision 2)
- [X] T005 [US1] skills/setup/SKILL.md 전체 OS 블록을 `Unix / Git Bash:` / `Windows PowerShell:` 라벨로 일관 분리 (FR-005, #160 컨벤션 미러). NEVER 절·D1 guard·위임 모델 산문은 불변

**Checkpoint**: Windows PowerShell 온보딩 전 단계 완주 가능 = MVP.

---

## Phase 4: User Story 2 — manifest apphub.yaml 정합 (Priority: P2)

**Goal**: Step 6 앱 감지를 canonical `apphub.yaml` 우선으로 정정.

**Independent Test**: `apphub.yaml` 디렉토리 = "앱 있음", 빈 디렉토리 = 첫 앱 제안 (spec US2-AC1~2).

> ⚠️ **cosmetic** — Step 6 은 OR 존재검사라 파일명 순서가 동작을 안 바꿔요. 산문 reorder + legacy `mv` 힌트 한 줄 이상으로 부풀리지 말 것 (advisor 가이드).

- [X] T006 [US2] skills/setup/SKILL.md Step 6 manifest 감지를 canonical-first 로 정정 — 산문에서 `apphub.yaml` 우선 언급, bash `test -f apphub.yaml`/`ls` + PS `Test-Path apphub.yaml` 명령 추가, legacy `axhub.yaml` 발견 시 `mv axhub.yaml apphub.yaml` 힌트 한 줄 (contracts Step 6, FR-004, research Decision 4)

**Checkpoint**: US1 + US2 독립 동작.

---

## Phase 5: User Story 3 — CLI v0.17.2 정합 유지 (Priority: P3)

**Goal**: 변경이 setup 의 CLI 표면·위임·helper 계약 정합을 깨지 않음을 보증 (회귀 방지).

**Independent Test**: 변경 후 setup 이 여전히 `axhub --version` 감지, install-cli/auth/init 위임, preflight `auth_ok`/`user_email` 읽기 동작 (spec US3-AC1).

- [X] T007 [US3] 변경 후 setup 이 참조하는 CLI 표면 정합 재확인 — `axhub --version`, 위임 `Skill("axhub:{install-cli,auth,init}")`, helper preflight `auth_ok`/`user_email` 가 그대로 유지되는지 skills/setup/SKILL.md diff 로 검토 (FR-007, research Decision 1)

**Checkpoint**: 전 스토리 독립 동작 + 정합 유지.

---

## Phase 6: Polish & Cross-Cutting

**Purpose**: 커버리지·검증 게이트·회귀

- [X] T008 [P] 블록 커버리지 확인 — skills/setup/SKILL.md 에서 `grep -c "Unix / Git Bash:"` == `grep -c "Windows PowerShell:"` (SC-002 갭 0)
- [X] T009 repo root 에서 검증 게이트 실행 (대상 변경 파일: skills/setup/SKILL.md) — `bun run skill:doctor --strict`(exit 0) · `bun run lint:tone --strict`(0 err) · `bun run lint:keywords --check`(no diff, description 불변) · `bun test`(회귀 0) · `bunx tsc --noEmit`(clean) (SC-004)
- [X] T010 quickstart.md 회귀 검증 — macOS/Linux bash 경로가 변경 전과 동일 동작 (FR-006). `pwsh` 있으면 PS 블록 구문 파싱도 확인

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 의존 없음, 즉시 시작
- **Foundational (Phase 2)**: 해당 없음
- **User Stories (Phase 3~5)**: 모두 같은 `skills/setup/SKILL.md` 편집이라 **순차** (US1 → US2 → US3)
- **Polish (Phase 6)**: 전 변경 후

### 단일 파일 제약

- T002~T007 은 동일 `skills/setup/SKILL.md` 편집 → **병렬 불가**, 순차.
- [P] 가능: T001(읽기 전용 레퍼런스), T008(grep 검사)만.

### Within Each User Story

- US1: Step 1 감지(T002) → Step 1 helper(T003) → Step 4 node(T004) → 라벨 일관화(T005)
- 각 task 후 부분 검증(skill:doctor) 권장

---

## Parallel Opportunities

- **T001** (레퍼런스 읽기) — 다른 task 와 독립, 선행 가능
- **T008** (커버리지 grep) — 변경 후 독립 검사
- 나머지 T002~T007 은 동일 파일이라 순차 (병렬 시 충돌)

---

## Implementation Strategy

### MVP First (User Story 1)

1. T001 레퍼런스 패턴 확정
2. T002~T005 (US1 Windows PowerShell)
3. **STOP & VALIDATE**: T008 커버리지 + T009 게이트 일부
4. MVP = Windows 온보딩 동작

### Incremental Delivery

1. US1 (T001~T005) → MVP
2. US2 (T006) cosmetic 최소
3. US3 (T007) 정합 재확인
4. Polish (T008~T010) 커버리지 + 게이트 + 회귀

---

## Notes

- 단일 파일 in-place — 대부분 순차. commit 은 US 단위 또는 전체 후.
- `description:` frontmatter 불변 (lint:keywords byte-lock) — Windows 지원은 **본문 명령 블록에만** 추가.
- **#160(upgrade Windows) 정확 미러** — 동일 라벨, helper-pick doctor 단순도, manifest ConvertFrom-Json. 새 패턴 발명 금지.
- **US2 부풀리지 말 것** (cosmetic — 산문 reorder + mv 힌트 한 줄).
- pwsh 런타임 미검증은 실재 한계 — 컨벤션 일치 = 검증 (PR #160 동일).

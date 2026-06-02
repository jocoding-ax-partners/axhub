# Implementation Plan: verify 스킬 ↔ ax-hub-cli v0.17.2 + axhub-helpers 계약 정렬

**Branch**: `feat/update-skill-cli-alignment` | **Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/006-verify-skill-cli-alignment/spec.md`

## Summary

`skills/verify/SKILL.md` 를 ax-hub-cli **v0.17.2** + `axhub-helpers` 실제 계약에 맞춰 정렬해요. 전체 audit(Clarifications Q2) 결과 핵심 사실:
- **helper verify 출력 `verdict:"passed"` 는 틀려요** — 실제 `VerifyResult.verdict` ∈ {`live`,`suspect`,`not_live`}. `reasons`/`last_deploy_id` 필드도 누락.
- **`deploy status .status` 는 닫힌 enum 이 아니라 백엔드 free string** (`Option<String>`). 정렬은 "enum 매칭"이 아니라 helper `LIVE_STATES`(`live/running/deployed/active/ok/succeeded`) 휴리스틱 미러 + 그 외=미라이브.
- **`deploy logs` 는 app-level** 로 바뀌었어요 (`list_app_logs`, deployment_id 는 "Legacy", "Runtime logs scoped to app-level backend route"). `--source` 는 free passthrough(pod/runtime/build enum 없음). 스킬의 `<DEPLOY_ID> --source pod` per-deploy pod-log 모델은 stale → `--app` app-level 로.
- **`--app-id`(primary)/`--app`(alias)** — 스킬 설명이 뒤집힘.
- **recover error_code 표** (skills/recover/SKILL.md §canonical) 는 현행 — verify 는 cross-link 만, 정정 불요.

audit 에서 **CLI 버그는 없었어요** → 이 작업은 **SKILL.md 문서 정렬 only** (helper Rust / ax-hub-cli 변경 불필요). 접근: verify 본문을 현 계약에 맞춰 rewrite, frontmatter `description:` byte 보존, in-body preflight 블록·D1 가드·TodoWrite Step 0 보존, repo skill 게이트 통과.

## Technical Context

**Language/Version**: Markdown SKILL authoring (`skills/verify/SKILL.md`). 검증 toolchain = Bun + TypeScript + cargo. 정렬 대상 = Rust ax-hub-cli **v0.17.2** + `axhub-helpers`(동일 repo).

**Primary Dependencies**: ax-hub-cli `deploy {status,list,logs}` surface + `axhub-helpers {verify,list-deployments,preflight}` + `verify_helper.rs`(VerifyResult/LIVE_STATES) + `skills/recover/SKILL.md`(error_code 표). repo toolchain — `skill:doctor`/`lint:tone`/`lint:keywords`/`bun test`/`tsc`/`cargo test`.

**Storage**: N/A (문서 정렬).

**Testing**: `bun run skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`, `bun test`, `bunx tsc --noEmit`. live `axhub deploy {status,list,logs} --help` + `axhub-helpers verify --help`/소스 대조. (verify_helper.rs 무변경이면 cargo 는 회귀 확인용.)

**Target Platform**: Claude Code skill 런타임이 사용자 머신의 axhub CLI + axhub-helpers 를 구동. 비대화형(CI/headless) 가드 포함.

**Project Type**: 단일 repo 문서/스킬 정렬.

**Performance Goals**: N/A.

**Constraints**:
- frontmatter `description:` byte-identical 고정 (`lint:keywords --check`).
- 본문 해요체 (`lint:tone --strict`).
- `needs-preflight: true` 유지 — in-body CANONICAL_PREFLIGHT_BLOCK 보존 (load-time `!command` 주입 금지).
- `multi-step: true` 유지. D1 비대화형 가드 + TodoWrite Step 0 보존.
- AskUserQuestion(`health_endpoint_setup`) registry 엔트리 유지 (`tests/fixtures/ask-defaults/registry.json`).

**Scale/Scope**: SKILL.md 1개(~134줄) 본문 rewrite. `skills/recover/SKILL.md`(error_code 표)는 **read-only 참조** — 안 건드림. helper/CLI 코드 변경 0.

**NEEDS CLARIFICATION**: 없음 — Clarifications 에서 drift-guard(별도 feature) + audit 깊이(전체) 해소. status enum/`--source` audit 는 Phase 0 research 에서 완료(아래).

## Constitution Check

*GATE: Phase 0 전 통과. Phase 1 후 재확인.*

`.specify/memory/constitution.md` = 미작성 템플릿 → 비준 gate 없음. de-facto governance = repo `CLAUDE.md` Skill Authoring 계약 + lint/test 게이트 (005 와 동일):

| Gate | 준수 |
|---|---|
| description byte-lock | ✅ 본문만 수정 |
| 해요체 lint:tone | ✅ |
| in-body preflight(needs-preflight:true) 보존 | ✅ CANONICAL_PREFLIGHT_BLOCK 유지 |
| scaffold 패턴(D1/TodoWrite) 보존 | ✅ |
| AskUserQuestion ↔ registry 동기화 | ✅ health_endpoint_setup 유지 |
| skill:doctor --strict | ✅ DoD |

**판정: PASS**. Complexity Tracking 불요.

## Project Structure

### Documentation (this feature)

```text
specs/006-verify-skill-cli-alignment/
├── plan.md              # 이 파일
├── spec.md              # specify + clarify 산출
├── research.md          # Phase 0 — verify/deploy/helper 계약 audit + 결정
├── data-model.md        # Phase 1 — VerifyResult + deploy 응답 + verdict 매핑
├── quickstart.md        # Phase 1 — rewrite + 검증 절차
├── contracts/
│   └── verify-cli-contract.md   # Phase 1 — verify/deploy 권위 계약
└── tasks.md             # /speckit-tasks 산출
```

### Source Code (정렬 대상)

```text
skills/verify/SKILL.md                       # 주 rewrite 대상 (오케스트레이션 본문 + verdict 매핑)
skills/recover/SKILL.md                      # error_code 표 — READ-ONLY 참조 (정본, 안 건드림)
crates/axhub-helpers/src/verify_helper.rs    # VerifyResult/LIVE_STATES — READ-ONLY (이미 옳음)
tests/fixtures/ask-defaults/registry.json    # health_endpoint_setup — AskUserQuestion 변경 시에만
```

**Structure Decision**: 단일 repo 문서 정렬. 주 변경 = `skills/verify/SKILL.md` 1개. audit 에서 CLI/helper 버그 없음 확인 → Rust 변경 0, 신규 스킬 0, 삭제 0. recover 표는 정본이라 참조만. ax-hub-cli + axhub-helpers 는 read-only 권위 원천.

## Complexity Tracking

> Constitution Check 위반 없음 → 해당 없음.

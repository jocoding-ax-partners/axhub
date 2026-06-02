# Implementation Plan: update 스킬 ↔ ax-hub-cli v0.17.2 계약 정렬

**Branch**: `worktree-mighty-squishing-finch` | **Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-update-skill-cli-alignment/spec.md`

## Summary

`skills/update/SKILL.md` 본문이 가정한 구버전 CLI 계약(없는 env `AXHUB_REQUIRE_COSIGN`/`AXHUB_ALLOW_UNSIGNED`/`AXHUB_DISABLE_AUTOUPDATE`, exit 2 오해석, 틀린 cosign subcode, brew 감지 분기)을 **ax-hub-cli v0.17.2 의 실제 `update` 계약** 으로 정렬해요. 정렬 근거는 전부 primary-source 로 확정됐어요: live `axhub update --help`, `crates/axhub-core/src/exit_code.rs` + `error.rs`, `axhub/src/commands/update.rs`, `docs/cli-exit-codes.md`. 접근은 "스킬 본문을 CLI 계약의 거울로 rewrite + 직접 참조된 error-empathy-catalog 의 cosign/exit 문구 정정" 이고, frontmatter `description:`(nl-lexicon 트리거)는 byte 보존, 본문 해요체 유지, repo skill 게이트 전부 통과를 DoD 로 둬요. drift-guard 와 brew generic note 는 spec Clarifications 에 따라 범위 밖이에요.

## Technical Context

**Language/Version**: Markdown SKILL authoring (`skills/update/SKILL.md`). 검증 toolchain = Bun + TypeScript. 정렬 대상 CLI = Rust **ax-hub-cli v0.17.2** (별도 repo, 여기서 빌드 안 함).

**Primary Dependencies**: ax-hub-cli v0.17.2 계약 — `axhub update {check,apply}` surface + `ax-hub-cli/docs/cli-exit-codes.md`. repo skill toolchain — `bun run skill:doctor`, `lint:tone`, `lint:keywords`, `bun test`, `bunx tsc`.

**Storage**: N/A (문서 정렬).

**Testing**: `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`(no diff), `bun test`(skill 회귀: ux-todowrite / ux-ask-fallback-registry 등), `bunx tsc --noEmit`. 보조로 live `~/.axhub/bin/axhub update --help` spot-check + 가공 명령 grep=0.

**Target Platform**: Claude Code skill 런타임이 사용자 머신(darwin/linux/windows)의 axhub CLI 를 구동. 비대화형(CI/headless) 가드 포함.

**Project Type**: 단일 repo 내 문서/스킬 정렬 (신규 스킬·삭제 없음).

**Performance Goals**: N/A.

**Constraints**:
- frontmatter `description:` byte-identical 고정 (`lint:keywords --check` baseline lock).
- 본문 한글 해요체 (`lint:tone --strict`).
- scaffold 우회 금지 — 기존 스킬 편집이라 패턴(D1 guard / TodoWrite Step 0 / frontmatter 선언)은 이미 존재, 보존만.
- `needs-preflight: false` 유지 (preflight 블록 없음), `multi-step: true`, `model: sonnet` 유지.
- AskUserQuestion 변경 시 `tests/fixtures/ask-defaults/registry.json` 동기화.

**Scale/Scope**: SKILL.md 1개(~160줄) 본문 rewrite + `skills/deploy/references/error-empathy-catalog.md` 의 cosign/exit 문구 정정(생성물 관계 확인 필요). 신규 파일·스킬 없음.

**NEEDS CLARIFICATION**: 없음 — spec `/speckit-clarify` 에서 2개(drift-guard 범위, brew 처리) 해소.

## Constitution Check

*GATE: Phase 0 전 통과 필수. Phase 1 후 재확인.*

`.specify/memory/constitution.md` 는 **미작성 템플릿**(`[PRINCIPLE_X]` placeholder)이라 비준된 프로젝트 헌법 gate 가 없어요. 따라서 이 feature 의 de-facto governance 는 repo `CLAUDE.md` 의 **axhub Skill Authoring 계약** + lint/test 게이트로 대체해요:

| Gate (de-facto) | 출처 | 이 feature 준수 |
|---|---|---|
| frontmatter description byte-lock | CLAUDE.md "NEVER description 변경" | ✅ 본문만 수정, description 불변 |
| 해요체 강제 | `lint:tone --strict` | ✅ 모든 신규 한글 해요체 |
| scaffold 패턴 보존 | CLAUDE.md "scaffold 우회 금지" | ✅ 기존 스킬, 패턴 존재·보존 |
| frontmatter `multi-step`/`needs-preflight`/`model` 선언 | CLAUDE.md Self-Check | ✅ 이미 선언됨(true/false/sonnet) |
| AskUserQuestion ↔ registry 동기화 | `ux-ask-fallback-registry` test | ✅ AskUserQuestion 추가/변경 시 registry 갱신 |
| skill:doctor --strict 통과 | CI | ✅ DoD |

**판정: PASS** (위반 없음). Complexity Tracking 불요.

## Project Structure

### Documentation (this feature)

```text
specs/005-update-skill-cli-alignment/
├── plan.md              # 이 파일
├── spec.md              # /speckit-specify + /speckit-clarify 산출
├── research.md          # Phase 0 — CLI 계약 사실 + gap 결정
├── data-model.md        # Phase 1 — 스킬이 분기하는 계약 엔티티(exit/subcode/JSON)
├── quickstart.md        # Phase 1 — rewrite + 검증 절차
├── contracts/
│   └── update-cli-contract.md   # Phase 1 — axhub update 권위 계약 (스킬 정합 대상)
└── tasks.md             # /speckit-tasks 산출 (이 명령은 생성 안 함)
```

### Source Code (정렬 대상)

```text
skills/update/SKILL.md                              # 주 rewrite 대상 (본문 + exit/subcode 매핑). stale subcode 2곳(115,152)
# --- error-empathy-catalog 정정은 codegen 체인 (research D10) ---
crates/axhub-helpers/data/catalog.json              # source-of-truth. stale 키 update.cosign_verification_failed. 변경 전 blast radius grep
skills/deploy/references/error-empathy-catalog.generated.md  # AUTO-GENERATED — 직접 편집 금지, `bun run codegen:catalog` 로 regen
skills/deploy/references/error-empathy-catalog.md   # hand-authored, skill 링크 대상. line 160 직접 수정
tests/codegen.test.ts                               # catalog.json ↔ generated.md drift 강제 — regen 누락 시 fail
skills/deploy/references/nl-lexicon.md              # 트리거 lexicon — 변경 불요(읽기 참조만)
tests/fixtures/ask-defaults/registry.json           # AskUserQuestion 변경 시에만 동기화
```

**Structure Decision**: 단일 repo 정렬. 신규 스킬 0, 삭제 0. 1차 대상은 `skills/update/SKILL.md`. catalog cosign subcode 정정은 **codegen 체인** 이라(직접 `.md` 편집 아님): `catalog.json`(source) 수정 → `bun run codegen:catalog` regen(`tests/codegen.test.ts` 가드) → hand-authored `error-empathy-catalog.md` 수정. **catalog.json 키 변경은 blast radius 가 있어**(helper Rust/다른 skill/subcode 매핑) tasks 착수 시 grep 으로 먼저 확인하고, 과하면 범위 재조정해요. ax-hub-cli(Rust)는 read-only 권위 원천이라 빌드/수정 안 해요.

## Complexity Tracking

> Constitution Check 위반 없음 → 해당 없음.

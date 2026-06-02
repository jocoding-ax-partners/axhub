# Implementation Plan: setup 스킬 Windows PowerShell 지원 + CLI v0.17.2 정합

**Branch**: `feat/setup-windows-powershell` | **Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-setup-windows-powershell/spec.md`

## Summary

setup 온보딩 스킬의 OS 의존 셸 명령을 Windows PowerShell 등가와 함께 제공해, Windows 첫 사용자도 bash 없이 온보딩을 완주하게 해요. CLI v0.17.2 독립 재검토 결과 명령 정합은 이미 OK 라, 실질 작업은 (a) Step 1/2/4/6 의 PowerShell 블록 추가, (b) manifest `apphub.yaml` canonical 정정이에요. 기존 bash·위임 모델·D1 guard 를 보존하고, sibling 스킬(install-cli/doctor/recovery/upgrade PR #160)의 검증된 cross-platform 패턴을 차용해요.

## Technical Context

**Language/Version**: Markdown (`skills/setup/SKILL.md`); 검증 tooling 은 Bun / TypeScript

**Primary Dependencies**: 위임 sibling 스킬(install-cli/auth/init), `axhub-helpers`(preflight), `ax-hub-cli` v0.17.2

**Storage**: N/A (영속 데이터 없음 — data-model.md 참조)

**Testing**: `skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`, `bun test`, `tsc --noEmit`

**Target Platform**: macOS / Linux / Windows (PowerShell + Git Bash)

**Project Type**: Claude Code skill (단일 SKILL.md + 차용 참조)

**Performance Goals**: N/A

**Constraints**: `description:` frontmatter byte-lock(lint:keywords 불변), `allows-dependency-execution: false`, D1 guard 보존, 위임 모델 유지, 해요체

**Scale/Scope**: 단일 `skills/setup/SKILL.md`(~175줄), OS 블록 4개 영역(Step 1 감지·Step 1 helper·Step 4 node·Step 6 manifest)

## Constitution Check

*GATE: Phase 0 전 통과, Phase 1 후 재확인.*

`.specify/memory/constitution.md` 는 미비준 template(placeholder)이라 정식 헌법 gate 가 없어요(N/A). 대신 프로젝트 실질 계약(`CLAUDE.md` axhub Skill Authoring)을 gate 로 적용해요:

- ✓ scaffold 우회 아님 — 기존 SKILL 수정(신규 생성 아님)
- ✓ `description:` nl-lexicon 불변 (lint:keywords byte-lock)
- ✓ 해요체 유지 (lint:tone)
- ✓ `multi-step: true` / `needs-preflight: false` / D1 guard 유지
- ✓ 검증 게이트 5종 적용

**판정**: 위반 0 → Phase 0/1 통과. (Phase 1 후 재확인: 설계가 단일 파일 in-place 라 새 위반 없음.)

## Project Structure

### Documentation (this feature)

```text
specs/004-setup-windows-powershell/
├── plan.md                          # 이 파일
├── research.md                      # Phase 0 (4 decisions)
├── data-model.md                    # Phase 1 (entity 없음 — 감지 상태만)
├── quickstart.md                    # Phase 1 (검증 방법)
├── contracts/
│   └── setup-command-matrix.md      # Phase 1 (OS 명령 블록 계약)
└── checklists/
    └── requirements.md              # /speckit-specify (16/16)
```

### Source Code (repository root)

```text
skills/setup/SKILL.md                # 유일 수정 대상 (in-place 리팩토링)

# 차용 참조 (수정 안 함 — 위임 모델)
skills/install-cli/SKILL.md          # OS 감지 $env:OS 패턴
skills/doctor/SKILL.md               # helper .exe 탐색 + cache-scan 폴백
skills/deploy/references/recovery-flows.md  # $env:USERPROFILE 경로 규칙
skills/upgrade/SKILL.md              # PR #160 PowerShell 블록 컨벤션
```

**Structure Decision**: 단일 `skills/setup/SKILL.md` in-place 리팩토링. 새 파일·디렉토리 없음. 위임 모델상 sibling 스킬 본문은 미수정. 새 코드/crate 없음 — markdown 명령 블록만 additive.

## Complexity Tracking

위반 없음 — 비움.

# Implementation Plan: Source-Inferred Tables & Env Recommendations (`infer-tables-env`)

**Branch**: `main` (spec dir `008-infer-tables-env`) | **Date**: 2026-06-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/008-infer-tables-env/spec.md`

## Summary

개발자가 만든 소스코드를 분석해서 이 앱이 필요로 하는 axhub 동적 테이블(컬럼·타입·제약 포함)과 환경변수 키를 근거와 함께 추천하고, 사용자가 승인하면 기존 `tables`·`env` 스킬로 **이동(handoff)** 해서 실제 생성/등록까지 닫아주는 새 SKILL `infer-tables-env`.

**아키텍처 결정(중요)**: 별도 Rust 추론 엔진을 만들지 않아요. 새 SKILL 이 LLM 으로 소스를 직접 분석(declarative 아티팩트 우선)하고, 적용은 기존 `tables`/`env` 스킬·CLI 로 위임해요. axhub 의 thin-layer/skill-composition 철학에 맞고, 다국어 소스 파서를 5개 크로스아치 바이너리로 유지하는 비용을 피해요. 트레이드오프(테스트 가능한 recall/precision 보장 약화)는 Complexity Tracking 에 기록해요.

## Technical Context

**Language/Version**: SKILL.md (Markdown) + 스캐폴드/테스트는 TypeScript on Bun. 새 Rust 코드 없음 — 기존 `axhub-helpers`(Rust) 는 preflight/consent-mint 만 read-only 재사용.

**Primary Dependencies**: `axhub` CLI (v0.17.x) · `axhub-helpers`(preflight, consent-mint) · 기존 `tables` SKILL + `env` SKILL · Claude Code/Desktop skill runtime · host TodoWrite/AskUserQuestion.

**Storage**: 없음(휘발성, FR-015). 영구 상태는 axhub backend 의 동적 테이블·env 값뿐.

**Testing**: `bun test`(skill-doctor, lint:tone, lint:keywords, ask-fallback-registry 회귀) · e2e fixture 앱(샘플 프로젝트 → 기대 추천) · routing corpus(트리거 키워드 충돌 검증).

**Target Platform**: Claude Code(CLI) + Claude Desktop(macOS/Windows/Linux), axhub 플러그인 경유.

**Project Type**: axhub 플러그인 SKILL(오케스트레이션). 모노레포 내 `skills/` 패밀리.

**Performance Goals**: 자동 제안(US3)은 무거운 전체 스캔 없이 경량 넛지(즉시). 명시 분석은 인터랙티브(수 초).

**Constraints**: hook fail-open(exit 0) · 시크릿 평문 비노출(FR-004/012, SC-005) · mutate 전 사람 승인(FR-006/007, SC-006) · 휘발성(FR-015) · env 값 비추론(FR-016).

**Scale/Scope**: 단일 프로젝트 디렉토리 분석. axhub 템플릿 스택 앱 1급, declarative 아티팩트 고신뢰.

## Constitution Check

*GATE: Phase 0 전 통과, Phase 1 후 재확인.*

`.specify/memory/constitution.md` 는 미작성 placeholder 라 구체 gate 없음 → **사실상의 governance = 프로젝트 CLAUDE.md 규칙**. 이 SKILL 은 아래를 따라야 통과:

- **Skill Authoring (Phase 17/18)**: `bun run skill:new infer-tables-env --model sonnet` 스캐폴드 사용(직접 작성 금지). frontmatter `multi-step: true` + `needs-preflight: true` + `model: sonnet`. in-body `CANONICAL_PREFLIGHT_BLOCK` 포함. TodoWrite Step 0 + D1 TTY guard. AskUserQuestion 마다 `tests/fixtures/ask-defaults/registry.json` 등록. step-numbering collision 없음.
- **Tone**: 모든 한글 해요체. `bun run lint:tone --strict` 0 err.
- **nl-lexicon**: 트리거 어구는 frontmatter `description:` 에만. 새 SKILL 추가 → `lint:keywords` 베이스라인 재캡처(rare event, 허용).
- **Hook Safety (Phase 25)**: 자동 제안(US3)을 hook 으로 붙이면 `hook_safety::is_hook_disabled` + fail-open. (MVP 는 hook 없이 description 트리거 + init/deploy 스킬 한 줄 넛지로 최소 구현 → hook 미도입 권장.)
- **Model Routing**: `sonnet`(multi-step/interactive/mutate 위임).

**Gate 평가**: 위 계약을 plan 이 모두 명시 → PASS. 위반 없음.

## Project Structure

### Documentation (this feature)

```text
specs/008-infer-tables-env/
├── plan.md              # 이 파일
├── research.md          # Phase 0 — 결정·근거
├── data-model.md        # Phase 1 — 추천 데이터 모델 + 타입 매핑
├── quickstart.md        # Phase 1 — 사용·빌드·테스트 흐름
├── contracts/
│   ├── recommendation-contract.md   # 분석 산출(추천) 구조 계약
│   └── apply-handoff-contract.md     # tables/env 위임 CLI + consent 계약
├── checklists/
│   └── requirements.md  # spec 품질 체크리스트(이미 통과)
└── tasks.md             # /speckit-tasks 산출(이 단계서 안 만듦)
```

### Source Code (repository root)

```text
skills/
└── infer-tables-env/
    └── SKILL.md                 # 신규(스캐폴드). LLM 분석 + tables/env 위임 오케스트레이션

tests/
├── fixtures/
│   ├── ask-defaults/
│   │   └── registry.json        # 수정: AskUserQuestion(분석/적용 분기) safe_default 등록
│   └── infer/                   # 신규: 샘플 앱 fixture(declarative 스키마/.env.example)
│       ├── nextjs-prisma/       #   → 기대 추천(table/env) 골든
│       └── fastapi-sqlmodel/
├── ux-*.test.ts                 # 기존 패턴 계약 회귀(자동 enforce, slug 하드코딩 금지)
└── e2e/                         # 선택: claude-cli 매트릭스에 분석→승인→적용 시나리오

# 재사용(신규 코드 없음):
#   crates/axhub-helpers   preflight / consent-mint (read-only 재사용)
#   skills/tables          테이블 생성·컬럼 위임 대상
#   skills/env             env 키 등록·stdin 값 입력 위임 대상
#   axhub CLI              tables list/create, env list/set --from-stdin
```

**Structure Decision**: 새 mutation 경로·새 바이너리 없이 `skills/infer-tables-env/SKILL.md` 하나를 추가하고, 분석은 SKILL 본문의 LLM 절차로, 적용은 기존 `tables`/`env` 스킬·CLI 로 위임해요. cross-check(이미 설정됨)와 preflight 는 기존 read-only CLI/helper 를 호출해요.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| LLM-기반 분석을 결정론 Rust 엔진 대신 채택 | 다국어 소스 파서(Prisma/Alembic/SQLAlchemy AST…)를 5개 크로스아치 바이너리로 신설·유지하는 비용이 큼. 소스 이해는 LLM 강점. 사람이 검토·승인하는 추천이라 자율 mutation 아님 | Rust 결정론 엔진은 recall/precision 을 유닛테스트로 보장하고 시크릿 비노출을 코드로 강제할 수 있으나(advisor 권고), 유지비·범위가 reviewed-recommendation 도구에는 과함. 대신 recall/precision 은 fixture 평가(SC-001/002)로, 시크릿 안전은 SKILL `NEVER` + env 마스킹 + 승인 게이트로 확보. (Rust 엔진은 V2 hardening 후보로 보류) |

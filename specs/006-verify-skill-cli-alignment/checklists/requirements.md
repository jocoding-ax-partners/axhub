# Specification Quality Checklist: verify 스킬 ↔ ax-hub-cli v0.17.2 + axhub-helpers 계약 정렬

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-06-02
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [~] Written for non-technical stakeholders — 의도적 deviation (아래 Notes 참조)
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [~] Success criteria are technology-agnostic (no implementation details) — 의도적 deviation (아래 Notes 참조)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- **`[~]` 범례 = 이 feature 유형에서 의도적으로 충족하지 않는 항목** (누락 아님). verify 는 "스킬 문서 ↔ 외부 CLI/helper 계약" 정렬이라, 정렬 대상인 명령·verdict enum·JSON 필드·status 값이 명세의 *주제* 예요.
  - **"Written for non-technical stakeholders"** — 비기술 독자는 "verdict ∈ {live,suspect,not_live}" / "deploy status .status 분기" 를 못 읽어요. 대상 독자는 스킬 작성자·CLI 통합 담당이라 기술 계약 용어가 필수예요.
  - **"Success criteria are technology-agnostic"** — SC-001(verdict 값), SC-005(cargo/bun 게이트)는 구현 특정적이에요. "스킬이 실제 helper/CLI 계약과 일치하는가" 는 그 계약을 명명해야 측정 가능해서 기술 특정 SC 가 정확성의 핵심이에요.
  - 두 항목은 기술 세부를 제거하지 않고 마킹만 정직하게 했어요.
- **[NEEDS CLARIFICATION] 0**: 확정 gap(verdict:"passed"→live, --app-id/--app primary/alias, LIVE_STATES `ok` 누락)은 primary-source 로 확인했고, 미확정 항목(deploy status `.status` 전체 enum, `--source` 허용값)은 plan/tasks 의 audit 대상으로 Assumptions/FR 에 명시했어요. scope 모호성은 없어요(005 와 동일 유형, verify 1개 스킬 한정).
- 005 (update) 와의 관계: 같은 "skill↔CLI 정렬" 패턴의 2번째. verify 는 orchestration(preflight/list-deployments/deploy status·list·logs/helper verify/health) 이 많아 audit 표면이 넓어요. 단 핵심 정정(verdict:"passed")은 SKILL.md 문서 수정이라 helper Rust 변경 불요.
- 모든 항목 통과 → `/speckit-clarify`(선택) 또는 `/speckit-plan` 진행 준비 완료.

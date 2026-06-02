# Specification Quality Checklist: update 스킬 ↔ ax-hub-cli v0.17.2 계약 정렬

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

- **`[~]` 범례 = 이 feature 유형에서 의도적으로 충족하지 않는 항목** (누락이 아니라 deliberate). 이 명세는 "스킬 문서 ↔ 외부 CLI 계약" 정렬이라, 정렬 대상인 명령·종료코드·subcode 가 명세의 *주제* 예요. 따라서:
  - **"Written for non-technical stakeholders"** — 비기술 독자는 "exit 66 enforce-blocked, `error.subcode` 로 구분" 을 읽지 못해요. 대상 독자는 스킬 작성자·CLI 통합 담당이라, 기술 계약 용어가 불가피하고 *필수* 예요. 그래서 비기술 가독성은 일부러 양보했어요.
  - **"Success criteria are technology-agnostic"** — SC-002(grep `AXHUB_REQUIRE_COSIGN`=0), SC-003(exit 14/15/66), SC-004(`bun run skill:doctor`) 는 구현 특정적이에요. 그러나 "스킬이 실재하는 CLI 계약과 일치하는가" 는 그 계약을 명명하지 않으면 측정 불가라서, 기술 특정 SC 가 정확성의 핵심이에요. 추상화하면 검증력이 사라져요.
  - 두 항목은 기술 세부를 *제거* 하지 않고 마킹만 정직하게 했어요 — 기술 내용은 정확하고 plan 단계에 필수예요.
- **brew/scoop 분기 제거(FR-007) 는 사용자 영향 있는 유일한 product 결정** — v0.17.2 `update` 가 `package_manager` 신호를 안 내보내도 brew 로 설치한 사용자는 존재할 수 있어요. 명세는 이를 **되돌릴 수 있는 assumption** 으로 잡았고(올바른 spec-level 처리), `/speckit-clarify` 나 사용자가 무게를 재고 싶을 수 있는 항목이에요. 나머지는 0 clarification 으로 충분해요.
- 이 기능은 본질적으로 "스킬 문서 ↔ CLI 계약" 정렬이라, 정렬 대상인 CLI 명령·종료코드는 명세의 *주제*예요. 명세는 "어떤 명령을 호출하라" 같은 구현 지시 대신, "스킬이 문서화하는 계약이 실제 CLI 계약과 일치해야 한다" 는 검증 가능한 요구로 표현했어요. 명령 이름·exit code 수치는 요구의 측정 대상으로만 등장하고, 본문 작성·코드 변경 방법(HOW)은 `/speckit-plan` 단계로 미뤘어요.
- `update` 관련 종료 코드의 행위별 정렬(14/15/66 subcode 등)은 SC-003 으로 측정하되, exit-code-by-exit-code 본문 재작성 자체는 plan/tasks 산출물이에요.
- 검증 가능성: ax-hub-cli v0.17.2 바이너리가 `~/.axhub/bin/axhub` 에 설치돼 있어 live 대조가 가능하고, 불가 시 `docs/cli-exit-codes.md` 가 대체 권위예요(Assumptions/Dependencies 에 명시).
- [NEEDS CLARIFICATION] 0 개: 사용자가 물을 뻔한 scope 모호성(좁은 update 정렬 vs 광범위 재정렬)은 `specs/002-skills-cli-alignment/refactor-plan.md` 재검토로 해소했어요 — 002 가 명령 존재만 검증하고 flag/exit-code parity 는 미검증으로 남겼고, 이 명세가 그 update 부분만 채워요.
- 모든 항목 통과 → `/speckit-clarify`(선택) 또는 `/speckit-plan` 진행 준비 완료.

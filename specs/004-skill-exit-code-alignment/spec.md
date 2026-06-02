# Feature Specification: Skill 복구 라우팅 ↔ 현행 CLI 실패 신호 정합

**Feature Branch**: `004-skill-exit-code-alignment`

**Created**: 2026-06-02

**Status**: Draft

**Input**: User description: "skills/status 의 스킬에 대해서 ax-hub-cli 바뀐 cli에 대해서 검증하고 맞지 않다면 리팩토링 계획 작성해줘"

> **검증 결론 (요약)**: `status` 스킬은 **명령·플래그 레벨에서는 현행 CLI 와 정합**해요 (선행 분석 `specs/002-skills-cli-alignment` 와 일치). 그러나 **실패 후 복구 라우팅**(CLI 가 보고하는 실패 조건 → 한국어 공감 안내)은 현행 CLI 가 더 이상 내보내지 않는 신호에 묶여 있어 **어긋나 있어요**. 이 라우팅은 `status` 단독이 아니라 공유 카탈로그(`skills/deploy/references/error-empathy-catalog.md`)에 있고, 8개 스킬이 함께 소비해요. 그래서 범위가 카탈로그-레벨이에요. 조건별 구체 신호 대조표는 동반 문서 `verification-report.md` 에 있어요.

## Clarifications

### Session 2026-06-02

- Q: 미처리 CLI 코드(`8`~`15`)의 복구 안내 범위는? → A: §verification-report 도달성 분석으로 실제 skill 경로에 도달하는 조건만 bespoke 4-part 템플릿을 쓰고, 도달하지 않거나 전용 안내가 없는 조건은 공통 "안전하게 멈췄어요" fallback 항목 하나로 라우팅해요.
- Q: 향후 drift 재발 방지 가드를 추가할까요? → A: CLI 실패-계약을 axhub repo 에 pinned snapshot 으로 vendoring 하고, 카탈로그 라우팅 키 집합이 그 snapshot 과 정확히 일치하는지 검증하는 자동 parity 테스트를 추가해요 (drift 시 CI fail).
- Q: 옛 CLI 와의 호환 처리는? → A: Hard-cut — 현행 0.17.2 계약(4/5/6/8/66...)에만 매핑하고 옛 65/67/68 은 제거해요. plan 첫 작업으로 ax-hub-cli git 이력(pickaxe)을 돌려 "옛 코드를 emit 한 release 가 있었나"를 확인 — 없으면 무손실 정답, 있었으면 dual-map / min-version 게이트로 에스컬레이션.

## User Scenarios & Testing *(mandatory)*

배경: axhub 의 핵심 사용자는 **11시 데모를 앞둔 불안한 바이브 코더**예요. 무언가 실패했을 때 임상적/엉뚱한 메시지를 받으면 포기 캐스케이드에 빠져요. 복구 카탈로그는 모든 실패 조건을 "감정 + 원인 + 행동 + 버튼" 4-파트 안내로 바꿔 사용자를 흐름 안에 붙잡아두려고 존재해요. 이 기능은 그 안내가 **실제로 발생한 실패에 정확히 매칭되도록** 카탈로그를 현행 CLI 실패 계약에 다시 맞추는 일이에요.

### User Story 1 - 로그인 만료 → 재로그인 안내 (Priority: P1)

토큰이 만료된 사용자가 상태/배포/로그를 요청하면, 스킬이 인증 만료를 알아채고 "다시 로그인해줘" 안내를 띄워줘야 해요. 지금은 인증 만료가 일반 통신 오류 템플릿("일시적 통신 문제, 다시 시도")으로 잘못 라우팅돼서, 사용자가 아무리 재시도해도 풀리지 않는 루프에 갇혀요.

**Why this priority**: 가장 빈번한 복구 가능 실패이고, 현재 오라우팅이 만료-토큰 사용자를 전원 무한 재시도에 가둬요. 카탈로그가 막으려는 포기-캐스케이드의 정확한 방아쇠예요.

**Independent Test**: `status` 스킬에 만료-토큰 실패를 주입하고, 일반 통신 템플릿이 아니라 재로그인 안내가 뜨는지 확인하면 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** 내 인증 토큰이 만료된 상태, **When** 배포 상태를 물어보면, **Then** 스킬이 인증 만료를 감지하고 재로그인을 제안해요.
2. **Given** 비대화형(CI/headless) 세션에서 토큰이 만료된 상태, **When** `status` 스킬이 돌면, **Then** 일반 재시도 경로가 아니라 등록된 안전 기본값(자동 로그인 안 함, abort)을 따라요.

### User Story 2 - 리소스 못 찾음 → did-you-mean 제안 (Priority: P1)

앱/배포 이름을 잘못 입력했을 때, 일반 실패가 아니라 가장 비슷한 후보를 제안하는 did-you-mean 안내를 받고 싶어요.

**Why this priority**: 흔한 오타 경로예요. did-you-mean 은 카탈로그의 시그니처 기능인데 not-found 에서 현재 전혀 발동하지 않아요.

**Independent Test**: 존재하지 않는 앱 이름으로 스킬을 돌려 후보 제안이 뜨는지 확인.

**Acceptance Scenarios**:

1. **Given** 없는 앱을 참조한 상태, **When** 스킬이 돌면, **Then** 가장 가까운 후보들을 보여줘요.

### User Story 3 - 정책 차단 vs 권한 부족 구분 (충돌 없음) (Priority: P1)

서로 다른 실패 조건은 서로 다른 안내를 내야 해요. 정책 차단(다운그레이드/cosign 검증 실패)이 일반 권한-부족 메시지와 같은 안내를 내거나, 그 반대가 되면 안 돼요. 지금은 이 두 조건이 한 라우팅 키를 공유해서 엉뚱한 템플릿이 발동할 수 있어요.

**Why this priority**: 공유 라우팅 키가 한 조건을 이중 부킹해서, 보안 관련(cosign) 안내가 엉뚱하게 나올 수 있어요. 안전 메시지 정확성 문제예요.

**Independent Test**: 정책-차단 실패와 권한-부족 실패를 각각 주입해 서로 다른 템플릿이 뜨는지 확인.

**Acceptance Scenarios**:

1. **Given** 정책 차단(설치 차단)과 권한-부족이라는 두 조건, **When** 각각 발생하면, **Then** 각각 자기 고유 템플릿으로 라우팅돼요 (키 공유 없음).

### User Story 4 - rate limit → 자동 backoff 안내 (Priority: P2)

서버가 요청을 제한하면, 일반 "한 번 재시도"가 아니라 서버가 알려준 시간만큼 기다렸다가 자동 재시도하라는 안내를 받고 싶어요.

**Why this priority**: 복구는 되지만 빈도·심각도가 P1 보다 낮아요. 단, 현재 rate-limit 도 일반 경로로 falling through 해요.

**Independent Test**: rate-limit 실패(재시도-지연 포함)를 주입해 대기·안내가 뜨는지 확인.

**Acceptance Scenarios**:

1. **Given** 서버가 재시도-지연과 함께 rate limit 을 보고한 상태, **When** 스킬이 그걸 만나면, **Then** 그 시간만큼 기다리고 사용자에게 알려줘요.

### User Story 5 - 미처리 실패 조건도 안전하게 (Priority: P2)

CLI 가 보고할 수 있지만 카탈로그에 항목이 없는 실패 조건(테넌트 범위, 타임아웃, 충돌, 도메인 차단, 초대 만료, 무결성 실패 등)은, 최소한 정직한 "안전하게 멈췄어요" 안내로 라우팅돼야 해요 — 엉뚱한 원인 메시지가 떠선 안 돼요.

**Why this priority**: 빈도는 낮지만, 미매핑 조건이 인접한 잘못된 템플릿을 띄우면 사용자를 더 혼란스럽게 해요. 정직성 안전망이에요.

**Independent Test**: 전용 템플릿이 없는 실패 조건을 주입해, 잘못된-원인 메시지가 아니라 정직한 fallback 이 뜨는지 확인.

**Acceptance Scenarios**:

1. **Given** 전용 템플릿이 없는 실패 조건, **When** 그게 발생하면, **Then** 사용자는 정직한 "안전하게 멈췄어요 + 실제 다음 행동" 안내를 받고, 절대 잘못된-원인 메시지를 받지 않아요.

### User Story 6 - 카탈로그 단일 출처 정합 (Priority: P1, 횡단)

공유 복구 카탈로그를 쓰는 모든 스킬은 같은 실패 조건을 동일하고 정확하게 라우팅해야 해요. 카탈로그/CLI 가 더 이상 합의하지 않는 라우팅 키를 쓰는 스킬이 하나도 없어야 해요.

**Why this priority**: 이게 근본이에요. `status` 는 단독으로 고칠 수 없고, 공유 카탈로그가 8개 스킬이 소비하는 단일 출처예요. 일부 스킬은 이미 새 신호로 부분 마이그레이션됐고 일부는 옛 신호에 남아 있어 불일치 상태예요.

**Independent Test**: 공유 카탈로그로 실패를 라우팅하는 각 스킬에서, 동일 CLI 실패 조건이 모두 같은 올바른 템플릿으로 가는지 대조.

**Acceptance Scenarios**:

1. **Given** 공유 카탈로그로 실패를 라우팅하는 임의의 스킬, **When** 특정 CLI 실패 조건이 발생하면, **Then** 그런 모든 스킬이 그걸 같은 올바른 복구 템플릿으로 매핑해요.
2. **Given** CLI 의 문서화된 실패 조건 계약, **When** 카탈로그를 감사하면, **Then** CLI 가 낼 수 있는 모든 조건은 카탈로그 항목이 정확히 하나씩 있고, CLI 가 낼 수 없는 조건을 참조하는 카탈로그 항목은 하나도 없어요.

### Edge Cases

- 한 실패 조건이 예약/사용법 신호와 같은 숫자 신호를 공유하면 → 숫자만으로 라우팅하지 말고 구조화된 원인(subcode)으로 구별해야 해요.
- 두 실패 조건이 정당하게 한 숫자 신호를 공유하되 subcode 로 갈리면 → 라우팅이 subcode 로 분기해야 해요.
- 비대화형 세션 → 사용자 프롬프트가 필요한 복구는 등록된 안전 기본값으로 대체돼야 해요.
- 이미 부분 마이그레이션된 스킬(새 신호 사용)과 옛 신호에 남은 스킬이 공존 → 정합 후 전부 일관돼야 해요.
- CLI 실패 계약이 다시 바뀌면 → 카탈로그가 계약 출처를 인용하고 있어 drift 를 감지할 수 있어야 해요.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 스킬 묶음은 인증-만료 실패를 감지하고 재로그인 복구 행동으로 라우팅해야 해요(MUST).
- **FR-002**: 스킬 묶음은 리소스-없음 실패를 감지하고 did-you-mean 제안 흐름으로 라우팅해야 해요(MUST).
- **FR-003**: 스킬 묶음은 rate-limit 실패를 감지하고, 서버가 준 지연을 존중하는 대기-후-재시도 흐름으로 라우팅해야 해요(MUST).
- **FR-004**: 전용 안내를 갖는 각 실패 조건은 정확히 하나의 고유 복구 템플릿에 매핑돼야 해요(MUST). 전용 안내를 갖는 서로 다른 두 조건이 한 라우팅 키를 공유해선 안 되고, 한 조건이 두 의미로 이중 부킹돼서도 안 돼요. (전용 안내가 없는 조건들이 공유하는 공통 fallback 항목은 FR-006 의 의도된 예외예요.)
- **FR-005**: 어떤 스킬도 현행 CLI 가 만들어낼 수 없는 실패 신호로 라우팅해선 안 돼요(MUST). 모든 라우팅 키는 CLI 의 문서화된 실패 계약 안의 조건에 대응해야 해요.
- **FR-006**: CLI 가 만들어낼 수 있는 모든 실패 조건은 정확히 하나의 복구 항목으로 라우팅돼야 해요(MUST). 실제 skill 경로에 도달하는 조건(`verification-report.md` 도달성 분석)은 전용 4-part 템플릿을 갖고, 도달하지 않거나 전용 안내가 없는 조건은 공통 "안전하게 멈췄어요" fallback 항목 하나로 라우팅돼요 — 절대 오해를 부르는 잘못된-원인 템플릿으로 가선 안 돼요.
- **FR-007**: 복구 카탈로그를 공유하는 스킬들은 동일 실패 조건을 동일하게 라우팅해야 해요(MUST, 단일 출처).
- **FR-008**: 비대화형 맥락에서 사용자 프롬프트가 필요한 복구는 등록된 안전 기본값을 대신 적용해야 해요(MUST).
- **FR-009**: 정합 작업은 모든 항목의 기존 4-파트 한국어 공감 템플릿 구조와 톤(해요체)을 보존해야 해요(MUST).
- **FR-010**: 변경은 기존 품질·회귀 게이트를 회귀 0건으로 통과해야 하고(MUST), 스킬의 트리거 발화(frontmatter 의 사용자-대면 트리거)는 보존돼야 해요. (구체 게이트 목록은 `verification-report.md` §7.)
- **FR-011**: 정합된 복구 카탈로그는 각 항목이 어느 CLI 실패-계약 출처에서 왔는지 인용해, 향후 CLI 변경 시 drift 가 한눈에 보이도록 해야 해요(MUST).
- **FR-012**: 작업은 CLI 실패-계약의 pinned snapshot 을 axhub repo 에 vendoring 하고, 카탈로그의 라우팅 키 집합이 그 snapshot 과 정확히 일치하는지 검증하는 자동 parity 가드를 추가해야 해요(MUST). CLI 계약이 바뀌면 가드가 fail 해서 drift 를 노출하고, 사람이 snapshot 을 재동기화하도록 강제해요.

### Key Entities *(include if feature involves data)*

- **실패 조건 (Failure condition)**: CLI 가 표면화하는 구별되는 복구-가능 상황(인증 만료, 없음, rate-limit, 정책 차단, 테넌트 범위, 타임아웃, 충돌 등). 메시지 텍스트와 무관하게 구조화된 신호(숫자 클래스 + 선택적 subcode)로 식별돼요.
- **복구 액션 / 공감 템플릿 (Recovery action / empathy template)**: 한 실패 조건에 대해 사용자가 받는 4-파트 한국어 안내(감정 + 원인 + 행동 + 버튼).
- **복구 카탈로그 (Recovery catalog)**: 실패 조건 → 복구 액션 매핑의 공유 단일 출처. 라우팅 스킬 전부가 소비해요.
- **CLI 실패 계약 (CLI failure contract)**: CLI 가 만들어낼 수 있는 실패 조건들의 권위 있는 목록(문서화된 exit-code SLA + 그 단일 출처로 인용되는 enum). 카탈로그가 따라야 할 진실의 출처예요.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: CLI 가 만들어낼 수 있는 실패 조건의 100% 가 정확히 하나의 복구 항목에 매핑돼요 (현재: 측정 가능한 일부가 오매핑되거나 미매핑).
- **SC-002**: 작업 도중 토큰이 만료된 사용자는 대화형 케이스의 100% 에서 재로그인 안내를 봐요 (현재: 0% — 일반 통신-오류 템플릿으로 falling through).
- **SC-003**: CLI 가 만들어낼 수 없는 조건을 참조하는 라우팅 키가 0개예요 (현재: 여러 개의 stale 키 존재).
- **SC-004**: 충돌 0건 — 어떤 실패 조건도 둘 이상의 의미로 해석되지 않고, 어떤 의미도 다른 의미와 키를 공유하지 않아요 (현재: 최소 1건 충돌).
- **SC-005**: 카탈로그를 공유하는 모든 스킬이 같은 실패 조건에 동일한 복구 안내를 내요 (현재: 최소 1개 스킬이 다른 신호 사용).
- **SC-006**: 정합 후 기존 검증 스위트(패턴/톤/키워드 lint, 테스트, 타입 체크, 라우팅 코퍼스 baseline)가 전부 통과해요 — 회귀 0건.
- **SC-007**: 유지보수자가 모든 복구 항목을 한 홉 만에 CLI 실패-계약 출처로 추적할 수 있어요 (카탈로그가 계약을 인용).
- **SC-008**: 카탈로그 라우팅 키와 CLI 실패-계약 사이의 임의 divergence 가 자동 가드에서 100% 감지돼 CI 가 fail 해요 (현재: 가드 없음 — drift 무탐지로 이번 버그가 잠복했음).

## Assumptions

- CLI 의 문서화된 실패 계약(exit-code SLA 문서 + 그 단일 출처로 인용되는 enum)은 CLI 0.17.2 기준 권위 있고 현행이에요.
- 검증은 CLI 0.17.2 기준으로 수행됐고, 정합 대상은 그 계약이에요. CLI 계약이 또 바뀌면 재검증이 필요해요(그래서 FR-011 이 인용 링크를 요구해요).
- 호환 정책은 **hard-cut** 이에요(Clarify Q3): 카탈로그는 현행 0.17.2 계약에만 매핑하고 옛 `65`/`67`/`68` 은 제거해요. 옛 코드를 실제 emit 한 ax-hub-cli release 가 있었는지는 plan 첫 작업의 git-이력(pickaxe) 검증으로 확정하고 — shipped 된 적 없으면 무손실, shipped 됐었으면 dual-map 또는 min-version 게이트로 에스컬레이션해요.
- 명령·플래그 레벨 정합은 이미 올바르다고 봐요(선행 분석 `specs/002-skills-cli-alignment` 가 확립). 이 기능은 복구-라우팅(실패 조건 → 공감) 정합으로만 한정돼요.
- `002` 에 기록된 `deploy create --branch` 3-자 모순은 별개의 범위-밖 cross-repo consent 결정이라 여기서 다루지 않아요.
- 조건별 구체 숫자 매핑은 동반 검증 보고서 `verification-report.md` 에 기록돼 있고, `/speckit-plan` 이 그걸 구체 편집으로 변환해요. 스펙을 행동-중심으로 유지하려고 숫자는 본문에서 의도적으로 뺐어요.

## Out of Scope

- 명령/플래그 리팩토링 (002 결과 no-op).
- `deploy create --branch` consent 설계 결정.
- 겹치는 스킬(status/verify/trace)의 중복 정리 — 별도 UX 결정.
- CLI 자체 변경 — 이 작업은 스킬을 CLI 계약에 맞추는 것이지 그 반대가 아니에요.

## Dependencies

- **복구 카탈로그 (2개 표면)**: hand-written `skills/deploy/references/error-empathy-catalog.md` + source-of-truth `crates/axhub-helpers/data/catalog.json` (→ `scripts/codegen-catalog.ts` 가 `error-empathy-catalog.generated.md` 생성). 이를 라우팅에 쓰는 8개 스킬(status·deploy·logs·recover·init·apps·update·auth).
- **Rust helper 라우팅 레이어**: `crates/axhub-helpers/src/list_deployments.rs`(EXIT_LIST_* + exit→slug 매핑) + `main.rs`(`classify-exit` 서브커맨드) — exit-code→복구 라우팅의 실제 코드 경로. (초기 spec 은 이 레이어를 누락했고, plan/research 가 scope 확장을 확정했어요.)
- CLI 실패 계약: `ax-hub-cli/docs/cli-exit-codes.md` + `crates/axhub-core/src/exit_code.rs` + `error.rs`(slug). drift-guard pinned snapshot `crates/axhub-helpers/data/cli-exit-contract.json`.
- 기존 테스트/lint 게이트. (라우팅 코퍼스 `tests/corpus*.jsonl` 는 exit-code 문자열을 포함하지 않아 이 정합과 무관 — research.md 결정 5.)

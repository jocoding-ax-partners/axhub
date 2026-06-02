# Feature Specification: verify 스킬 ↔ ax-hub-cli v0.17.2 + axhub-helpers 계약 정렬

**Feature Branch**: `feat/update-skill-cli-alignment`

**Created**: 2026-06-02

**Status**: Draft

**Input**: User description: "verify 스킬도 cli에 맞게 리팩토링 해줘"

## 배경 (왜 필요한가)

`skills/verify/SKILL.md` 는 사용자가 "방금 거 진짜 됐어 / 라이브 됐어 / 검증해 / smoke test" 라고 할 때, 배포가 실제 라이브인지 **evidence 기반 verdict 한 줄**(✅ 라이브 / ⚠️ 의심 / ❌ 안 됨)로 답하는 스킬이에요. 이 스킬은 여러 표면을 오케스트레이션해요: helper `preflight` → `list-deployments` → `axhub deploy status/list/logs` → `axhub-helpers verify` → (선택) health endpoint curl.

직전의 `update` 스킬 정렬(`specs/005-update-skill-cli-alignment`)과 같은 종류의 drift 가 verify 에도 있어요. 실제 ax-hub-cli **v0.17.2** + `axhub-helpers` 계약을 primary-source(live `--help`, `crates/axhub-helpers/src/verify_helper.rs`, `cli/args/mod.rs`, `ax-hub-cli/axhub/src/commands/deploy/*`)로 확인한 결과:

- **helper verify 출력 JSON 이 틀려요.** 스킬의 CI 예시(`{"state":"live","last_deploy_age_secs":120,"errors":[],"verdict":"passed"}`)에서 `verdict` 값이 **`"passed"`** 인데, 실제 `VerifyResult.verdict` enum 은 `live` / `suspect` / `not_live` (snake_case) 예요. `"passed"` 는 존재하지 않아요. 또 실제 출력의 `reasons`(한국어 사유 배열)·`last_deploy_id` 필드를 스킬이 안 다뤄요.
- **`--app-id` / `--app` 설명이 뒤집혀 있어요.** 스킬은 "`--app-id 도 alias 로 지원해요`" 라고 하는데, 실제는 `--app-id` 가 primary, `--app` 가 `visible_alias` 예요 (helper `verify` / `list-deployments` 둘 다).
- **live-state 목록이 불완전해요.** helper 의 `LIVE_STATES` 는 `live/running/deployed/active/ok/succeeded` 인데 스킬은 `active/succeeded/live/running/deployed` 만 적어 `ok` 를 빠뜨렸어요.
- **`deploy status` 의 `.status` 분기 집합이 미검증이에요.** 스킬은 `pending/building/deploying/stopped` 등을 가정하는데, 실제 CLI 가 그 값들을 내는지 대조가 안 됐어요(확인된 값: `succeeded`, `failed`).
- **`deploy logs --source pod` 의 `pod` 값이 미검증이에요.** 실제 `--source` 는 자유 문자열(`Option<String>`)이고 허용값(pod/runtime/build 등)이 CLI/백엔드에서 어떻게 해석되는지 확인이 필요해요.
- **`axhub verify` 메인 명령은 존재하지 않아요** (exit 64). 스킬은 올바르게 `axhub-helpers verify` 를 써요 — 이건 정상.

이 drift 는 "verdict 가 사실과 다르게 나오거나(틀린 JSON 파싱), 죽은 플래그/오해 소지 문서로 사용자·CI 가 잘못된 신뢰를 갖는" 위험이에요. 이 기능은 verify 스킬(+ 그 스킬이 직접 참조하는 문구)을 실제 계약에 맞춰 정렬해요.

## Clarifications

### Session 2026-06-02

- Q: 이 feature 에 재-drift 방지 guard 를 넣을까요? → A: 안 넣음 — 재-drift guard 는 update+verify(+향후 스킬)를 커버하는 **별도 공용 feature** 로 분리해요. 006 은 verify 정렬에만 집중해요. 2번 연속 drift(update→verify)라 재발 차단은 그 공용 feature 에서 근본적으로 해요.
- Q: `deploy status .status` 값 + `deploy logs --source` 값을 plan 에서 어디까지 audit 할까요? → A: **전체 audit** — status 값 전수(free string) + `--source` 허용값을 live CLI 로 검증·정렬하고 스킬을 현 CLI 에 맞춰요. 불일치가 CLI 쪽 버그면 verify 는 현 CLI 에 맞추고 CLI fix 는 별도 cross-repo PR 로 분리해요.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - verdict 가 실제 helper 출력과 일치 (Priority: P1)

사용자가 "방금 거 진짜 됐어?" 라고 하면, 스킬은 `axhub-helpers verify` 와 deploy 신호를 모아 verdict 를 보여줘요. 스킬이 문서화·파싱하는 helper JSON 모양이 실제 `VerifyResult` 와 일치해야, CI(JSON 소비자)와 스킬 둘 다 올바른 verdict 를 내요.

**Why this priority**: verify 의 핵심 산출물이 verdict 인데, 문서화된 출력(`verdict:"passed"`)이 실제 값(`live`)과 달라요. JSON 소비자(CI)가 `"passed"` 를 기대하면 항상 실패하고, 스킬도 잘못된 필드를 파싱해요.

**Independent Test**: 실제 `axhub-helpers verify --json --app-id <app>` 출력 스키마(`verdict` ∈ {live,suspect,not_live}, `state`, `last_deploy_id`, `last_deploy_age_secs`, `errors`, `reasons`)와 스킬이 문서화한 모양을 대조하면 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** 라이브 배포, **When** verify 실행, **Then** 스킬은 helper `verdict:"live"` 를 ✅ 라이브 verdict 로 매핑하고, `reasons` 배열을 verdict 아래에 그대로 보여줘요.
2. **Given** helper 가 `verdict:"suspect"`/`not_live` 반환, **When** verify 실행, **Then** ⚠️ 의심 / ❌ 안 됨 으로 정확히 매핑해요.
3. **Given** CI 가 `axhub-helpers verify --json` 파이프, **When** 출력 파싱, **Then** 문서화된 필드명·값이 실제 출력과 1:1 이에요.

---

### User Story 2 - 모든 명령·플래그가 실제 CLI 에서 수용 (Priority: P2)

스킬이 구동하는 모든 명령이 v0.17.2 + helper 가 받아들이는 것이어야 해요 — `axhub deploy status/list/logs` 의 플래그(positional `[DEPLOYMENT_ID]`, `--app`, `--json`, `--source`, `--follow`), helper `verify`/`list-deployments` 의 인자(`--app-id`/`--app`, `--limit`, `--json`), 그리고 `deploy status .status` 분기 값.

**Why this priority**: 명령이 거부되면(없는 플래그/값) verdict 자체를 못 내요. P1 의 출력 매핑이 맞아도 입력 명령이 깨지면 무의미해요.

**Independent Test**: 스킬이 문서화한 각 명령을 live v0.17.2 로 구동(또는 `--help` 대조)했을 때 전부 수용되고("unknown command/argument" 0건), `.status` 분기 값이 실제 CLI 가 내는 값의 부분집합이면 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** 최근 배포, **When** `axhub deploy status <id> --app <app> --json` 실행, **Then** 수용되고 `.status`/`.current_stage` 를 읽어요.
2. **Given** `deploy logs`, **When** `--source <value>` 지정, **Then** 그 값이 실제 CLI 가 인식하는 source 예요 (pod 가 무효면 유효값으로 교체).
3. **Given** helper verify, **When** `--app-id` 또는 `--app` 로 호출, **Then** 둘 다 수용돼요 (primary/alias 관계가 문서와 일치).

---

### User Story 3 - 오해 소지 문서·죽은 가정 제거 (Priority: P3)

스킬 본문이 실제 계약과 어긋나는 설명을 남기지 않아요 — `--app-id`/`--app` primary/alias 관계 정정, live-state 목록에 `ok` 포함, `--source` 값 유효화, helper error_code 라우팅(`../recover/SKILL.md` 표)과의 일관성.

**Why this priority**: 정확성의 마무리. P1/P2 가 동작을 맞추면, P3 는 잘못된 설명이 남아 다음 편집자·사용자를 오도하지 않게 청소해요.

**Independent Test**: 본문의 플래그/상태값/error_code 설명을 실제 계약과 대조해 불일치 0건이면 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** 최종 본문, **When** `--app-id`/`--app` 설명 확인, **Then** "primary `--app-id`, alias `--app`" 로 정확해요.
2. **Given** 최종 본문, **When** live-state 목록 확인, **Then** helper `LIVE_STATES` 와 일치(`ok` 포함)해요.

---

### Edge Cases

- **최근 배포 없음**: helper sentinel `{"state":"unknown","last_deploy_id":null}` → ❌/안내 후 종료.
- **5s timeout**: status/logs 가 hang → "의심" verdict (스킬이 이미 명시 — 유지).
- **deploy status 진행 중**(`pending`/`building`/`deploying` 류) → ⚠️ 의심 (값이 실제 CLI 와 일치해야).
- **health endpoint 미설정** → AskUserQuestion(비대화형이면 `health_endpoint_setup` safe_default=skip).
- **helper/CLI 부재·인증 실패** → `auth_error_code` 라우팅(cli_not_found→install-cli 등) + `../recover/SKILL.md` error_code 표.
- **deploy logs `--follow`/watch** → 비-TTY/agent 컨텍스트에서 단일 스냅샷 degrade (CLI 동작 확인 필요).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 스킬이 문서화·파싱하는 `axhub-helpers verify` JSON 은 실제 `VerifyResult` 스키마와 일치해야 해요 — `verdict` ∈ {`live`,`suspect`,`not_live`}, `state`, `last_deploy_id`, `last_deploy_age_secs`, `errors`, `reasons`. CI 예시의 `verdict:"passed"` 는 `verdict:"live"` 로 정정해야 해요.
- **FR-002**: 스킬은 helper `verdict` 3값을 verdict 표시(✅/⚠️/❌)로 정확히 매핑하고, `reasons` 배열을 verdict 아래에 그대로 노출해야 해요.
- **FR-003**: 스킬이 구동하는 `axhub deploy {status,list,logs}` 명령·플래그는 v0.17.2 가 수용하는 것만 써야 해요 — `status [DEPLOYMENT_ID] --app --json [--watch --watch-interval]`, `list --app --json` (no `--limit`), `logs [DEPLOYMENT_ID] --app --json --source <v> [--follow]`.
- **FR-004**: helper `verify`/`list-deployments` 호출은 실제 인자와 일치해야 해요 — primary `--app-id`, alias `--app`; `list-deployments` 의 `--limit`. 본문 설명도 이 primary/alias 관계로 정정.
- **FR-005**: `deploy status .status` 는 백엔드 **free string** 이에요 (닫힌 CLI enum 아님 — research D2). live-state 판정은 helper `LIVE_STATES`(`live/running/deployed/active/ok/succeeded`)와 일관돼야 하고, 그 외(진행중/실패 류 — `pending/building/failed/stopped` 등은 **휴리스틱 라벨이지 CLI enum 아님**)는 미라이브로 분류해요. plan/tasks 가 **live CLI 로 실제 status 값을 확인**(전수)해서 스킬 분기를 LIVE_STATES 휴리스틱에 정렬해야 해요. 불일치가 CLI 버그면 verify 는 현 CLI 에 맞추고 CLI fix 는 별도 cross-repo (Clarifications 2026-06-02).
- **FR-006**: `deploy logs --source` 에 쓰는 값은 실제 CLI/백엔드가 인식하는 source 여야 해요. plan/tasks 가 **live CLI 로 허용값을 전수 확인** 하고 (`pod` 가 무효면 유효값으로 교체) 정렬해야 해요. 불일치가 CLI 버그면 별도 cross-repo (Clarifications 2026-06-02).
- **FR-007**: helper error_code 분기는 `../recover/SKILL.md` 의 canonical `error_code` 표(`transport.cli_missing`/`auth.token_invalid`/`response.invalid_json` 등)와 일관돼야 해요.
- **FR-008**: in-body preflight 블록(needs-preflight:true 의 CANONICAL_PREFLIGHT_BLOCK)·D1 비대화형 가드·TodoWrite Step 0 패턴은 보존해야 해요.
- **FR-009**: frontmatter `description:` 트리거 어휘는 byte 단위로 보존(nl-lexicon baseline lock)하고, 본문 한글은 해요체를 유지해야 해요.
- **FR-010**: 스킬이 문서화한 명령·출력·상태값은 설치된 live v0.17.2 + `axhub-helpers`(불가 시 소스 계약)로 검증 가능해야 해요.

### Key Entities *(계약 관여)*

- **verify 스킬 문서 (`skills/verify/SKILL.md`)**: 정렬 대상. frontmatter(트리거·multi-step:true·needs-preflight:true) + 오케스트레이션 워크플로 본문.
- **helper verify 계약 (`crates/axhub-helpers/src/verify_helper.rs`)**: `VerifyResult`(verdict/state/last_deploy_id/last_deploy_age_secs/errors/reasons) + `LIVE_STATES` + sentinel. 출력 스키마의 단일 진실 원천.
- **helper CLI args (`cli/args/mod.rs`)**: `verify`/`list-deployments` 의 `--app-id`(primary)/`--app`(alias)/`--limit`/`--json`.
- **CLI deploy 계약 (ax-hub-cli v0.17.2 `deploy status/list/logs`)**: 플래그 + `.status`/`.current_stage` 값 + `--source`.
- **연계 참조**: `../recover/SKILL.md`(error_code 라우팅 표), `../deploy/references/{error-empathy-catalog,nl-lexicon}.md`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 스킬이 문서화한 helper verify JSON 필드·값이 실제 `VerifyResult` 와 **100% 일치**(특히 `verdict` 값 ∈ {live,suspect,not_live}, `"passed"` 0건).
- **SC-002**: 스킬이 구동하는 CLI/helper 명령·플래그의 **100%** 가 live v0.17.2 + `axhub-helpers` 에서 수용돼요("unknown command/argument" 0건).
- **SC-003**: `deploy status .status` 분기 값과 live-state 목록이 실제 계약(`LIVE_STATES` 포함)의 부분집합/일치 — 어긋난 값 0건.
- **SC-004**: 본문 grep 으로 `verdict.*passed` 0건, `--app-id`/`--app` 설명이 실제 primary/alias 와 일치.
- **SC-005**: 저장소 스킬 게이트 전부 통과 — `skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`(no diff), `bun test`, `bunx tsc --noEmit`, `cargo test -p axhub-helpers`(verify_helper 영향 시).

## Assumptions

- verify 스킬은 별도 `ax-hub-cli` 사용자 바이너리(v0.17.2, `~/.axhub/bin/axhub`) + 동일 repo 의 `axhub-helpers` 바이너리를 구동해요. 둘 다 dev 환경에서 검증 가능.
- 정렬의 단일 진실 원천 = live `--help` + `verify_helper.rs`(VerifyResult/LIVE_STATES) + `cli/args/mod.rs` + ax-hub-cli `deploy/*` 소스. 서로 다르면 live 바이너리 + 소스를 우선.
- `verdict:"passed"` 정정은 **SKILL.md 문서 수정**이에요 — helper 는 이미 `live`/`suspect`/`not_live` 를 정확히 emit 하므로 Rust 변경은 (현재로선) 불필요. (status 값/`--source` audit 결과에 따라 helper/CLI 측 변경이 필요하면 그건 별도 cross-repo 작업으로 분리.)
- frontmatter `description:`(트리거)는 byte 동일 유지. 변경은 워크플로 본문 + 직접 참조 문구에 한정.
- `axhub verify` 메인 명령 부재는 의도된 설계(verify=helper) — 스킬은 그대로 helper 를 써요.

## Out of Scope

- `ax-hub-cli`(Rust) 자체의 변경 — 이 작업은 스킬 문서 정렬이 1차. status 값/`--source` 가 CLI 측 버그로 판명되면 별도 cross-repo 작업.
- `axhub-helpers` 의 `verify_helper.rs` 로직 변경 — helper 출력이 이미 옳으면 손대지 않아요. (스키마 자체가 틀린 게 아니라 스킬 *문서* 가 틀림.)
- update / 기타 스킬 — 005 (update) 와 별개. 이 spec 은 verify 만.
- verify 와 무관한 deploy/status/logs 스킬의 독립 정렬.
- **재-drift 방지 guard** — 스킬 문서 ↔ CLI/helper 계약 자동 대조 test/CI 는 이번 feature 에 미포함. update+verify(+향후 스킬)를 커버하는 **공용 drift-guard 를 별도 feature 로 분리** (Clarifications 2026-06-02). 재발 위험은 인지하고 그 feature 에서 근본 차단해요.

## Dependencies

- ax-hub-cli **v0.17.2** + `axhub-helpers` 계약. 계약이 바뀌면 정렬도 다시.
- `../recover/SKILL.md` 의 error_code 라우팅 표(verify 가 참조).
- 저장소 스킬 toolchain(`skill:doctor`/`lint:tone`/`lint:keywords`/`bun test`/`tsc`/`cargo test`).

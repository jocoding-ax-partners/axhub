# Feature Specification: update 스킬 ↔ ax-hub-cli v0.17.2 계약 정렬

**Feature Branch**: `worktree-mighty-squishing-finch`

**Created**: 2026-06-02

**Status**: Draft

**Input**: User description: "/Users/wongil/Desktop/work/jocoding/ax-hub-cli 여기 cli에 맞게 update 스킬 리팩토링해줘"

## 배경 (왜 필요한가)

`skills/update/SKILL.md` 는 사용자가 "업데이트해줘 / 새 버전 있어 / brew upgrade" 같이 말할 때 활성화돼서, Claude 가 axhub CLI 버전 확인·업그레이드를 대신 실행해주는 스킬이에요. 그런데 이 스킬 본문은 **구버전(0.9.x 시절) 으로 가정된 CLI 계약** 에 맞춰 작성돼 있어요. 실제 사용자 바이너리는 별도 repo `ax-hub-cli` 의 **v0.17.2** 이고, `update` 명령 계약이 크게 달라졌어요:

- 스킬이 호출하는 `AXHUB_REQUIRE_COSIGN=1` 환경변수는 **CLI 에 존재하지 않아요** (cosign 검증은 기본 정책으로 항상 켜져 있고 env 토글이 없어요).
- 스킬이 안내하는 `AXHUB_ALLOW_UNSIGNED=1` 우회 env 도 **존재하지 않아요**.
- 스킬이 "exit 2 = 회사 autoupdate 정책 disable" 로 해석하는데, 실제로 **exit 2 는 clap usage(잘못된 인자) 전용 예약 코드** 예요 (`docs/cli-exit-codes.md` 가 "Do NOT remap" 명시). `AXHUB_DISABLE_AUTOUPDATE` env 도 CLI 에 없어요.
- 스킬의 brew/scoop 패키지매니저 감지(`exit 1 + package_manager:"brew"`) 분기는 현재 CLI 가 그런 신호를 내보내지 않아요 (v0.14.0+ 는 `~/.axhub/bin` 에 바이너리를 self-manage).
- 새 tamper/실패 코드인 **exit 14(digest mismatch)**, **exit 15(binary swap failed)** 를 스킬이 전혀 다루지 않아요.
- 스킬의 "other non-zero → `65/68/1`" 라우팅(Step 7)도 stale 예요 — 실제 계약(`exit_code.rs`)엔 **exit 65/68 이 없어요**. 미인증(exit 4)도 안 다뤄요.
- downgrade 우회는 실제로 `--force` 플래그(cosign 은 절대 안 건드림)인데 스킬은 이를 모르고 존재하지 않는 unsigned env 를 언급해요.

이 drift 는 단순 문구 문제가 아니라, **사용자가 업데이트를 요청했을 때 Claude 가 CLI 가 거부하는 명령(없는 env/flag)을 실행하거나, 보안 관련 실패(서명 검증 실패·변조 탐지)를 잘못 해석할 위험** 이에요. 직전의 광범위 정렬 작업(`specs/002-skills-cli-alignment/refactor-plan.md`)은 "모든 스킬의 **명령 존재**는 검증했지만 per-subcommand **flag/exit-code parity 는 미검증**" 이라고 명시적으로 남겼고, update 스킬의 env/exit-code 계약이 바로 그 미검증 영역이에요. 이 기능은 그 갭만 좁혀요.

## Clarifications

### Session 2026-06-02

- Q: update 스킬의 재-drift 방지 장치(문서 명령/exit-code ↔ CLI 계약 자동 대조 테스트)를 이 feature 에 포함할까요? → A: 포함 안 함 — 이번 feature 는 스킬 본문 정렬에만 집중하고, drift-guard(`axhub update --json-schema` 대조 등)는 별도 follow-up 으로 미뤄요. 재발 위험은 인지하되 수용해요.
- Q: brew/scoop 패키지매니저 감지 분기(`exit 1 + package_manager:"brew"`)를 어떻게 할까요? → A: 완전 제거 — v0.17.2 가 해당 신호를 안 내보내므로 dead branch 로 통째 삭제하고, 패키지매니저 관련 안내(generic 재설치 note 포함)도 두지 않아요.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 업그레이드가 실제 CLI 에서 그대로 동작 (Priority: P1)

사용자가 "axhub 업데이트해줘" 라고 하면, Claude 가 스킬을 따라 버전을 확인하고 동의를 받은 뒤 업그레이드를 적용해요. 이때 스킬이 실행하는 모든 명령·플래그가 실제 v0.17.2 CLI 가 받아들이는 것이어야 하고, 서명 검증은 그대로 유지된 채 업그레이드가 성공해야 해요.

**Why this priority**: 스킬의 존재 이유 그 자체예요. 지금은 `AXHUB_REQUIRE_COSIGN=1 axhub update apply ...` 처럼 CLI 에 없는 env 를 앞에 붙여 실행해서, 사용자가 가장 흔히 쓰는 경로가 깨지거나 잘못된 가정 위에서 돌아가요.

**Independent Test**: 설치된 axhub v0.17.2 바이너리로 dry-run → execute 흐름을 구동했을 때, 스킬이 문서화한 명령이 전부 수용되고("unexpected argument"/"unknown env" 류 거부 없음), cosign 서명 검증이 기본으로 enforce 됨을 확인하면 단독으로 검증돼요.

**Acceptance Scenarios**:

1. **Given** 새 버전이 있는 상태, **When** 사용자가 업그레이드에 동의, **Then** 스킬은 `axhub update apply --dry-run --json` 으로 먼저 미리보기를 보여주고, 이어서 `axhub update apply --execute --yes --json` (불필요한 env 접두 없이) 를 실행해 성공으로 끝나요.
2. **Given** 이미 최신 버전, **When** 버전 확인 실행, **Then** 스킬은 "이미 최신 버전이에요" 라고 안내하고 멈춰요.
3. **Given** CI/headless(`--non-interactive` / `$CI` / `$CLAUDE_NON_INTERACTIVE`) 환경, **When** 업데이트 흐름 진입, **Then** 스킬은 AskUserQuestion 을 건너뛰고 안전 기본값(적용 안 함)으로 멈춰요.

---

### User Story 2 - 종료 코드·결과를 정확히 해석 (Priority: P2)

CLI 가 0 이 아닌 코드로 끝났을 때, 스킬은 그 exit code 와 JSON 봉투의 `error.subcode` 를 **권위 계약(`docs/cli-exit-codes.md`)** 에 맞춰 정확한 사용자 메시지·다음 행동으로 매핑해요. 특히 새 코드인 14(digest mismatch)·15(swap failed) 와, 66 의 downgrade vs cosign subcode 구분을 올바로 처리해요.

**Why this priority**: 잘못된 해석은 보안 사고로 이어질 수 있어요(변조 탐지를 일반 오류로 오인하거나, 존재하지 않는 우회를 권하는 등). P1 의 happy path 가 정확해도 실패 경로 해석이 틀리면 위험해요.

**Independent Test**: update 와 관련된 각 종료 코드(0, 1, 4, 10, 14, 15, 64, 66)에 대해, 스킬이 문서화한 대응이 `docs/cli-exit-codes.md` 의 규정 행동과 1:1 로 일치하는지 대조하면 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** 다운로드 산출물 SHA256 이 릴리스 매니페스트 핀과 불일치(exit 14), **When** apply 실행, **Then** 스킬은 변조 신호로 보고 **즉시 중단**, 강제 진행을 절대 권하지 않고 IT/보안팀 통보를 안내해요.
2. **Given** 원자적 바이너리 교체 실패(exit 15), **When** apply 실행, **Then** 스킬은 **자동 재시도하지 않고**(바이너리가 부분 교체됐을 수 있음) 복구 경로를 안내해요.
3. **Given** downgrade 차단(exit 66, subcode = downgrade), **When** 사용자가 구버전 설치를 원함, **Then** 스킬은 cosign 을 건드리지 않는 `--force` 를 정확한 우회로 안내해요.
4. **Given** cosign enforce 실패(exit 66, subcode = cosign), **When** apply 실행, **Then** 스킬은 **하드 스톱**, 어떤 우회도 제시하지 않아요.

---

### User Story 3 - 존재하지 않는 명령·env·우회 제거 (Priority: P3)

스킬은 CLI 가 지원하지 않는 환경변수·플래그·분기를 사용자에게 절대 안내하지 않아요. 죽은 지시(`AXHUB_REQUIRE_COSIGN`, `AXHUB_ALLOW_UNSIGNED`, `AXHUB_DISABLE_AUTOUPDATE`, brew 감지 분기)를 제거하고, 실재하는 계약(`--force`, 기본 cosign, exit 14/15/66 subcode)으로 대체해요.

**Why this priority**: 정확성·신뢰의 마무리예요. P1/P2 가 올바른 경로를 깔면, P3 는 잘못된 경로가 본문에 남아 사용자를 오도하지 않도록 청소해요.

**Independent Test**: 스킬 본문에서 존재하지 않는 env/flag 토큰을 grep 하면 0 건이어야 하고, `--force` 설명이 `axhub update apply --help` 의 의미("downgrade 게이트만 우회, cosign 불변")와 일치하면 단독 검증돼요.

**Acceptance Scenarios**:

1. **Given** 최종 스킬 본문, **When** `AXHUB_REQUIRE_COSIGN` / `AXHUB_ALLOW_UNSIGNED` / `AXHUB_DISABLE_AUTOUPDATE` 검색, **Then** 매치 0 건.
2. **Given** 최종 스킬 본문, **When** package-manager(brew/scoop) 감지 분기 검색, **Then** 분기·관련 안내 0 건(완전 제거).

---

### Edge Cases

- **이미 최신**: `check` 가 `has_update:false` → "이미 최신 버전이에요" 후 정지.
- **동시 실행 락**: `~/.axhub/bin/axhub.update.lock` 존재(다른 apply 진행 중) → CLI 가 락으로 보호. 스킬은 자동 재시도하지 말고 안내만.
- **네트워크/타임아웃(exit 10)**: check/apply 중 전송 실패 → 일반 재시도 안내(단, apply 전송 실패는 자동 재시도 금지).
- **다운그레이드 의도**: 사용자가 구버전을 원할 때 `--force` 가 cosign 안전 우회임을 명확히, unsigned 우회와 혼동 금지.
- **부분 교체 후 롤백**: swap 실패 시 `~/.axhub/bin/axhub.<old>.bak` 백업이 있을 수 있음을 복구 안내에 반영.
- **인증 필요(exit 4)**: 업데이트 흐름 중 토큰 문제면 `axhub auth login` 으로 유도.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 스킬은 axhub v0.17.2 가 실제로 정의한 `update` 하위 명령·플래그만 사용해야 해요 — `update check`, `update apply` (+ `--dry-run` / `--execute` / `--yes` / `--force`), 전역 `--json`. 그 외 가공된 명령·플래그 사용 금지.
- **FR-002**: 스킬은 CLI 가 정의하지 않은 환경변수를 설정·권유하면 안 돼요. 구체적으로 `AXHUB_REQUIRE_COSIGN`, `AXHUB_ALLOW_UNSIGNED`, `AXHUB_DISABLE_AUTOUPDATE` 를 본문에서 제거해야 해요.
- **FR-003**: apply 단계는 env 토글이 아니라 **CLI 기본 정책(default-on)** 의 서명 검증에 의존해야 하고, `--execute` 전에 반드시 `--dry-run` 미리보기를 먼저 보여줘야 해요.
- **FR-004**: 스킬은 update 관련 종료 코드를 권위 계약(`docs/cli-exit-codes.md`)대로 해석해야 해요 — 0 성공, 1 일반 실패, **4 미인증(`axhub auth login` 유도)**, 10 타임아웃, **14 digest mismatch(변조→중단, force 금지)**, **15 swap failed(자동 재시도 금지)**, 64 usage, **66 enforce-blocked(downgrade vs cosign 을 `error.subcode` 로 구분)**. 현 스킬의 stale `65/68/1` 라우팅(Step 7)은 실제 계약 코드(1/4/10/64)로 교체해야 해요 — exit 65/68 은 계약에 없어요.
- **FR-005**: downgrade 차단(exit 66, downgrade subcode)에서 스킬은 `--force` 를 cosign 안전 우회로 안내해야 하고, `--force` 를 서명 검증 우회 수단으로 제시하면 절대 안 돼요.
- **FR-006**: cosign enforce 실패(exit 66, cosign subcode) 또는 digest mismatch(exit 14)에서 스킬은 하드 스톱하고 IT/보안팀 통보를 안내하며, 어떤 우회도 제시하면 안 돼요.
- **FR-007**: 스킬은 brew/scoop 패키지매니저 감지 분기(`exit 1 + package_manager:"brew"`)와 관련 안내를 **통째로 제거** 해야 해요 — v0.17.2 는 해당 신호를 내보내지 않아 dead branch 이고, generic 재설치 note 도 남기지 않아요 (Clarifications 2026-06-02).
- **FR-008**: 스킬은 frontmatter `description:` 트리거 어휘를 **byte 단위로 그대로 보존**해야 하고(nl-lexicon baseline lock), 본문 모든 한글은 해요체를 유지해야 해요.
- **FR-009**: 스킬은 비대화형(CI/headless) 가드를 유지하고, 안전 기본값을 "apply 안 함" 으로 두며, 이는 AskUserQuestion fallback registry 와 일관돼야 해요.
- **FR-010**: 스킬이 문서화한 모든 명령과 exit-code 매핑은 설치된 live `axhub` v0.17.2 (불가 시 `docs/cli-exit-codes.md`) 에 대해 검증 가능해야 해요.
- **FR-011**: 스킬이 파싱하는 JSON 필드는 CLI 의 실제 봉투와 일치해야 해요 — `check` 의 `{current, latest, has_update}` 와 오류 봉투의 `error.subcode` 를 live `--json` 출력으로 확인.

### Key Entities *(데이터/계약 관여)*

- **update 스킬 문서 (`skills/update/SKILL.md`)**: 정렬 대상. frontmatter(트리거·model·multi-step 등)와 워크플로 본문으로 구성. 본문이 CLI 계약의 거울이어야 해요.
- **CLI update 계약**: ax-hub-cli v0.17.2 의 `update check`/`update apply` 하위 명령, 플래그(`--dry-run`/`--execute`/`--yes`/`--force`), JSON 봉투(`current`/`latest`/`has_update`, `error.subcode`), 그리고 종료 코드. **단일 진실 원천**.
- **종료 코드 SLA (`ax-hub-cli/docs/cli-exit-codes.md`)**: exit↔의미↔subcode 매핑의 권위 표. 14/15/66 의 의미와 subcode dispatch 가 여기 박혀 있어요.
- **연계 참조 문서**: 스킬이 링크하는 `skills/deploy/references/error-empathy-catalog.md`(exit 66/cosign 템플릿), `nl-lexicon.md`(트리거). exit-code/cosign 문구가 stale 한 범위에서만 함께 갱신.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 스킬이 지시하는 CLI 명령·플래그의 **100%** 가 axhub v0.17.2 에서 수용돼요(dry-run 구동 시 "unexpected argument"/unknown-env 실패 0 건).
- **SC-002**: 스킬 본문에 CLI 에 없는 env/flag(`AXHUB_REQUIRE_COSIGN`, `AXHUB_ALLOW_UNSIGNED`, `AXHUB_DISABLE_AUTOUPDATE`) 참조가 **0 건**(grep count = 0).
- **SC-003**: `docs/cli-exit-codes.md` 의 update 관련 종료 코드(0,1,4,10,14,15,64,66) 각각에 대해 (스킬↔contract §3 수동 교차대조로) 스킬의 문서화된 대응이 **정확히 하나** 있고, 모두 계약의 규정 행동과 일치해요.
- **SC-004**: 저장소 스킬 작성 게이트 전부 통과 — `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`(no diff), `bun test`, `bunx tsc --noEmit`.
- **SC-005**: "axhub 업데이트해줘" 요청이, 스킬이 가공한 명령으로 인한 CLI 거부 없이 성공적 업그레이드(또는 올바른 안전 정지)에 도달해요.

## Assumptions

- 이 스킬은 **axhub 플러그인/스킬 repo(현재 repo)** 에 살면서 별도 `ax-hub-cli` 사용자 바이너리를 구동하는 방법을 문서화해요. 정렬 대상은 ax-hub-cli **v0.17.2** 이고, 개발 환경에 `~/.axhub/bin/axhub` 로 설치돼 있어요(검증 가능).
- 정렬의 단일 진실 원천 = live `axhub update --help` + `ax-hub-cli/docs/cli-exit-codes.md`(+ README 종료코드 섹션). 서로 다르면 live 바이너리와 exit-codes 문서를 우선해요.
- 이 작업은 광범위 다중 스킬 정렬(`specs/002-skills-cli-alignment`)이 "전부 keep, 명령 존재 검증" 으로 결론 내리며 **명시적으로 미검증으로 남긴 flag/exit-code parity** 의 update-스킬 부분이에요. 002 의 결론과 모순되지 않아요.
- 패키지매니저(brew/scoop) 감지 분기는 **완전 제거** 로 확정됐어요 (Clarifications 2026-06-02): v0.17.2 가 `~/.axhub/bin` 에서 self-manage 하고 패키지매니저 신호를 안 내보내 dead branch 예요. 향후 패키지 설치가 재도입되면 그때 별도로 재평가해요.
- frontmatter `description:`(nl-lexicon 트리거)는 byte 동일하게 유지하고, 변경은 워크플로 본문·exit-code 처리·직접 참조된 공용 exit-code/cosign 문구에 한정해요.
- downgrade 우회 `--force` 는 cosign 안전(`apply --help` 명시): downgrade 게이트만 우회하고 서명 검증은 절대 우회하지 않아요.

## Out of Scope

- `ax-hub-cli` 자체(Rust 소스)의 변경. 이 작업은 스킬 문서 정렬만 해요.
- 광범위 다중 스킬 재정렬 — 002 가 no-op 으로 결론.
- `deploy create --branch` consent 3-way 불일치(002 부록) — update 와 무관한 별개 cross-repo/consent 보안 결정.
- update 스킬과 무관한 다른 스킬의 flag drift 감사.
- **재-drift 자동 방지 장치(drift-guard)** — 스킬↔CLI 계약 자동 대조 테스트/CI 체크(`axhub update --json-schema` 활용)는 별도 follow-up. 이번 feature 는 일회성 정렬만 하고 재발 위험은 인지·수용해요 (Clarifications 2026-06-02).

## Dependencies

- ax-hub-cli **v0.17.2** 계약(설치 바이너리 + `docs/cli-exit-codes.md`)에 의존. 계약이 다시 바뀌면 정렬도 다시 해야 해요.
- 저장소 스킬 작성 도구체인(`bun run skill:doctor` / `lint:tone` / `lint:keywords` / `bun test` / `tsc`)이 동작해야 검증 가능.
- 검증 단계에서 live `axhub` 바이너리 가용성(없으면 exit-codes 문서로 대체).

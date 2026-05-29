# Feature Specification: axhub-helpers clap 리팩토링

**Feature Branch**: `001-helpers-clap-refactor`

**Created**: 2026-05-29

**Status**: Draft

**Input**: User description: "현재 axhub-helpers가 그냥 코딩으로 되어 있는데 clap으로 리팩토링하고 싶어"

## 배경 *(non-normative)*

`axhub-helpers` 바이너리의 진입점(`crates/axhub-helpers/src/main.rs`, 약 3,300줄)은 손수 작성한 argument 파싱으로 동작해요. `match cmd.as_str()` 한 덩어리로 약 50개 subcommand 를 dispatch 하고, subcommand 마다 `while i < args.len()` 루프로 flag 를 직접 분해해요. 사용법(`USAGE`)은 별도 하드코딩 상수라 명령 grammar 와 따로 관리돼서 drift 가 생기기 쉬워요. 이 명세는 그 진입점을 declarative 한 명령 정의(`clap`)로 옮기되, **외부에서 관측되는 동작은 한 바이트도 바꾸지 않는 것**을 목표로 해요.

`clap` 은 이미 workspace dependency 로 선언돼 있지만 소스에서는 쓰이지 않아요(주석 1곳 제외). 즉 이번 작업은 "부분 도입 마무리"가 아니라 "제로에서 clap 채택"이에요. 새 런타임 의존성 추가는 없어요.

## Clarifications

### Session 2026-05-29

- Q: per-command 으로 큐레이션된 한국어 help/error 콘텐츠(예: `routing-stats` PRIVACY 블록, `consent-mint` stdin 안내, `post-install` 한국어 에러)를 clap 영어 auto-help 로 대체할까요? → A: 한국어 콘텐츠 보존 — clap 의 custom `about`/`long_help` + 에러 커스터마이즈로 그대로 옮겨요. clap 영어 auto-help 는 top-level `--help` 배너 + 종합 usage-error(unknown subcommand) 에만 허용해요.
- Q: clap 도입 중 subcommand 이름·별칭·flag·positional 을 바꿀 수 있나요, 동결할까요? → A: 동결 + 버그성 불일치만 수정 — 이름·별칭·flag·positional 을 보존하는 순수 구조 마이그레이션이되, USAGE↔dispatch 간 명백한 버그성 불일치(오타 flag, 문서 누락 명령 등)는 별도 추적 + parity 무영향 조건으로 정정할 수 있어요.
- Q: fail-open(파싱 실패해도 exit 0) 경계 — 경계 명령(`autowire-statusline`, `karpathy-inject`, `token-gate`)을 fail-open 으로 볼까요? → A: plan 분류 채택 — `autowire-statusline`·`karpathy-inject` 는 fail-open(각각 SessionStart wrapper·prompt-route 임베드라 hook 맥락), `token-gate` 는 Normal(SKILL deploy gate — exit 0/65 unauthorized 신호 보존, parse error→exit 64). hooks.json + 호출 맥락 권위. (analyze F1/F2 해소.)

## User Scenarios & Testing *(mandatory)*

> 이 기능의 "사용자"는 axhub-helpers 를 유지보수하는 **개발자**와, 이 바이너리를 호출하는 **Claude Code hook / SKILL / 셸 래퍼**예요. 각 user story 는 subcommand 그룹 단위 슬라이스라 독립적으로 마이그레이션·검증·배포할 수 있어요. 우선순위는 production 위험도 순서예요(hook 경로가 가장 위험).

### User Story 1 - hook 진입점 + 파서 기반 마련 (Priority: P1)

clap 파서 골격을 세우고, hook 이 호출하는 subcommand(권위 set: `session-start`, `prompt-route`, `preauth-check`, `commit-gate`, `tdd-inject`, `classify-exit`, `test-classifier`, `state-update`, `autowire-statusline`)와 `version`/`help` 를 먼저 clap 으로 옮겨요. 이 슬라이스는 **fail-open 계약**(어떤 실패에서도 exit 0)을 보존하는 게 핵심이에요. (`karpathy-inject` 도 fail-open 이지만 hidden 이라 P3 에서 typed 이관 — 단 fail-open 분류는 파서 골격 단계에서 보장.)

**Why this priority**: hook 경로는 production 에서 매 세션·매 도구 호출마다 실행돼요. clap 의 기본 동작(파싱 실패 시 exit 2)이 그대로 새면 hook 이 메인 흐름을 차단해서 조용히 망가져요. 가장 위험한 표면을 먼저, 안전망을 깐 상태로 옮겨요.

**Independent Test**: hook subcommand 들을 정상 입력·잘못된 입력·알 수 없는 flag 로 호출해서 (1) 항상 exit 0, (2) stdout 의 hook JSON 형태가 동일, (3) `version --quiet` 가 어느 인자 순서에서도 빈 stdout+stderr+exit 0 임을 `hook_safety_cli.rs` / `version_quiet_test.rs` 로 확인해요. 이 슬라이스만 끝나도 production hook 안전성이라는 가치를 단독으로 전달해요.

**Acceptance Scenarios**:

1. **Given** hook subcommand 에 알 수 없는 flag 를 넘긴 상태, **When** 바이너리를 실행하면, **Then** clap 파싱이 실패해도 프로세스는 exit 0 으로 끝나고 hook 출력 계약을 깨지 않아요.
2. **Given** `version --quiet` 와 `--version --quiet` 두 인자 순서, **When** 각각 실행하면, **Then** 둘 다 빈 stdout·빈 stderr·exit 0 이에요.
3. **Given** `--version`(quiet 없음), **When** 실행하면, **Then** stdout 이 `axhub-helpers ` 로 시작하고 `schema v0` 을 포함해요.
4. **Given** `AXHUB_DISABLE_HOOKS=1` 등 kill switch 가 설정된 상태, **When** hook subcommand 를 실행하면, **Then** 기존과 동일하게 즉시 exit 0 으로 빠져나가요.

---

### User Story 2 - 사용자 직접 호출 subcommand (Priority: P2)

개발자/사용자가 직접 실행하거나 SKILL 이 호출하는 데이터·변경 계열 subcommand(`deploy-prep`, `sync`, `snippet`, `config get|set`, `verify`, `trace`, `doctor`, `bootstrap`(+`dependency-plan`), `consent-mint`/`consent-verify`, `token-init`/`token-import`/`token-gate`, `resolve`, `preflight`, `settings-merge`)를 clap 으로 옮겨요.

**Why this priority**: 직접 호출 빈도가 높고 exit code(0/64/65/70)와 JSON 출력이 SKILL·셸 래퍼·테스트 계약으로 잠겨 있어요. P1 의 파서 골격 위에서 진행하면 위험이 낮아져요.

**Independent Test**: 각 subcommand 의 정상·오류 입력을 `cli_e2e.rs`, `data_layer_cli.rs`, `deploy_prep_test.rs`, `settings_merge.rs`, `bootstrap_*` 로 돌려 exit code 와 JSON 바이트 동일성을 확인해요.

**Acceptance Scenarios**:

1. **Given** 필수 flag 누락(예: `deploy-prep` 의 `--intent`), **When** 실행하면, **Then** 기존과 동일한 exit 64 로 끝나요(clap 기본 2 가 아니라).
2. **Given** `config get <key>` / `config set <key> <value>` / `bootstrap dependency-plan` 같은 중첩 명령, **When** 실행하면, **Then** 기존 nested dispatch 와 동일하게 동작해요.
3. **Given** stdin 으로 JSON binding 을 받는 `consent-mint`, **When** 빈/잘못된 stdin 을 주면, **Then** 기존과 동일한 exit code(65) 와 에러 메시지 계약을 유지해요.

---

### User Story 3 - 분석·유지보수 subcommand (Priority: P3)

나머지 분석·운영 subcommand(`routing-stats`, `cleanup-audit`, `audit-clarify`, `routing-dashboard`, `list-deployments`, `mark`, `emit-deploy-complete`, `path`, `post-install`, `diagnose hitl`, `orphan-stub`, `auth-refresh-bg`, `redact`, `statusline`)와 문서에 없는 hidden subcommand(`state-show`, `consent`, `karpathy-inject`)의 typed 이관을 옮겨요. (`state-update` 는 hook 이라 P1 에서 이관 — 단 hidden 처리는 동일하게 적용.)

**Why this priority**: 호출 빈도·위험도가 가장 낮고, 앞 두 슬라이스에서 확립한 패턴을 그대로 적용해요. hidden subcommand 는 `USAGE` 에 노출되지 않던 동작을 그대로 유지해야 해요.

**Independent Test**: `routing-stats` duration 파싱(`1d`/`7d`/`all`), `path <kind>` positional, `emit-deploy-complete [exit_code [class]]` optional positional, hidden subcommand 호출을 각 테스트(`audit_e2e.rs`, `recovery_scan_test.rs`, `diagnose_*`, `post_install_test.rs`)로 확인해요.

**Acceptance Scenarios**:

1. **Given** `path token-file|last-deploy-file|state-dir` positional 인자, **When** 실행하면, **Then** 기존 경로 출력·exit code 가 동일해요.
2. **Given** 문서에 없는 hidden subcommand(`state-show --json` 등), **When** 실행하면, **Then** 기존과 동일하게 동작하고 `--help` 목록에는 노출되지 않아요.
3. **Given** `routing-stats --since all`, **When** 실행하면, **Then** 기존 duration 파싱과 동일한 결과를 내요.

---

### Edge Cases

- **hook 경로의 clap 파싱 실패**: clap derive 는 기본적으로 파싱 실패 시 exit 2 로 자동 종료해요. hook subcommand 에서는 이게 새면 안 돼요 → 반드시 가로채서 exit 0(fail-open)으로 변환해야 해요. **가장 중요한 회귀 위험.**
- **subcommand 없이 호출**: 현재는 `USAGE` 를 stderr 로 찍고 exit 64. clap 기본은 exit 2 → 64 로 remap 필요.
- **`-v`/`-h` 단축**, **`version`/`--version`/`-v` 3중 별칭**, **`help`/`--help`/`-h`** 모두 보존.
- **stdin + flag 혼합**: `classify-exit` 는 stdin 이 있으면 payload 경로, 없으면 `--exit-code`/`--stdout` flag 경로로 분기해요. 두 경로 모두 보존.
- **조건부 stdin**: `bootstrap` 은 `--record apps_create|deploy_create` 일 때만 stdin 을 읽어요.
- **optional positional**: `emit-deploy-complete [<exit_code> [<command_class>]]` 의 0/1/2 인자 케이스.
- **UTF-8 console (Windows)**: 진입 시 codepage 설정이 clap 도입 후에도 동일하게 먼저 실행돼야 해요.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001 (동작 parity)**: System MUST 모든 subcommand 의 외부 관측 동작을 보존해야 해요 — 프로세스 자체 exit code(0/64/65)와 JSON 보고값(70/124/127 = shelled-out `axhub` CLI·data-layer 결과, data-model §3), stdout/stderr 분리, 그리고 stdout 의 JSON 출력 바이트 동일성. 이 동작들은 테스트·hooks.json·SKILL·셸 래퍼 계약으로 잠겨 있어요.
- **FR-002 (fail-open 보존, 최우선)**: Claude Code hook 이 호출하는 subcommand 는 clap 파싱 실패를 포함한 **어떤 실패에서도 exit 0** 이어야 해요. clap 의 기본 exit-2-on-parse-error 가 이 경로로 새면 안 돼요. 권위 set(hooks.json + SessionStart wrapper, Clarifications 2026-05-29 확정):
  - **core 8개**: `session-start`(SessionStart exec), `prompt-route`(UserPromptSubmit), `preauth-check`·`commit-gate`(PreToolUse Bash), `tdd-inject`(PreToolUse Edit\|Write), `classify-exit`·`test-classifier`(PostToolUse[+Failure] Bash), `state-update`(PostToolUse Edit\|Write `--edit-event`).
  - **추가**: `autowire-statusline`(SessionStart wrapper 경유), `karpathy-inject`(`prompt-route` 임베드 — hook 맥락).
  - **예외**: `token-gate`(SKILL deploy gate, 등록명 `token-freshness-gate`)는 exit 0/65(unauthorized=65) 의미를 보존하므로 일반(Normal) 분류 — parse error 는 exit 64.
- **FR-003 (단일 grammar, 핵심 목표)**: System MUST 각 명령의 argument grammar 를 declarative 한 한 곳(clap 명령 정의)에서 선언해야 해요. subcommand 별 손수 작성 `while i < args.len()` flag 루프는 제거돼야 해요.
- **FR-004 (version --quiet)**: System MUST `version --quiet` / `--version --quiet` 를 인자 순서와 무관하게 빈 stdout·빈 stderr·exit 0 으로 처리하고, quiet 없을 때는 `axhub-helpers ...` 배너에 `schema v0` 을 포함해야 해요.
- **FR-005 (usage-error exit code remap)**: System MUST usage 오류(알 수 없는 subcommand·flag, 필수 인자 누락, subcommand 미지정)를 기존 exit 64 로 매핑해야 해요(clap 기본 2 가 아니라).
- **FR-006 (top-level help/error 텍스트)**: System MAY 개발자/사람 대상 top-level `--help` 레이아웃과 종합 usage-error 문구(unknown subcommand 등)를 clap 자동 생성 형태(영어 가능)로 채택할 수 있어요. 이때 명령 grammar 와 자동 동기화돼 help drift 가 사라져요. 이는 하드코딩 `USAGE` 상수를 대체해요.
- **FR-006a (per-command 한국어 콘텐츠 보존)**: System MUST per-command 으로 큐레이션된 한국어 help/error 콘텐츠를 보존해야 해요 — `routing-stats` 의 PRIVACY 안내 블록, `consent-mint` 의 stdin 가이드, `post-install`·`routing-stats` 등의 한국어 에러 메시지. 이들은 clap 의 custom `about`/`long_help` + 에러 커스터마이즈로 옮겨야 하고, clap 의 terse 영어 auto-help 로 퇴화시키면 안 돼요.
- **FR-007 (hidden subcommand 보존)**: System MUST 문서에 없던 subcommand(`state-show`, `state-update`, `consent`, `karpathy-inject`)의 동작을 보존하고, 이들이 `--help` 목록에 노출되지 않도록 hidden 처리해야 해요. (`state-update`·`karpathy-inject` 는 hook 으로도 동작하지만 USAGE 미노출이라 hidden 유지.)
- **FR-008 (중첩·positional 구조 보존)**: System MUST 중첩/positional 명령 구조를 보존해야 해요 — `config get|set`, `bootstrap dependency-plan`, `diagnose hitl`, `path <kind>`, `mark <phase>`, `emit-deploy-complete [exit_code [class]]`.
- **FR-009 (stdin 계약 보존)**: System MUST stdin 을 읽는 subcommand 의 입력 계약을 보존해야 해요(`redact`, `consent-mint`, `consent-verify`, `classify-exit`, `commit-gate`, `test-classifier`, `tdd-inject`, `prompt-route`, `preauth-check`, `token-import`, 조건부 `bootstrap --record`).
- **FR-010 (kill switch·hook-safety 보존)**: System MUST 기존 kill switch / hook-safety env 계약(`AXHUB_DISABLE_HOOKS`, `AXHUB_DISABLE_HOOK`, legacy `DISABLE_AXHUB` 등)을 그대로 유지하고, clap 도입이 이 분기를 우회하지 않아야 해요.
- **FR-011 (의존성·외부 계약 불변)**: System MUST 새 런타임 의존성을 추가하지 않아야 하고(`clap` 은 이미 존재), `hooks/hooks.json`·`hooks/*.sh`·`hooks/*.ps1`·SKILL invocation 을 수정 없이 그대로 동작시켜야 해요.
- **FR-012 (진입 시 부수동작 순서 보존)**: System MUST 진입 직후 부수동작(Windows UTF-8 console codepage 설정 등)이 인자 파싱 전에 기존 순서대로 실행되도록 보존해야 해요.
- **FR-013 (scope 동결)**: System MUST subcommand 이름·별칭(`version`/`--version`/`-v`, `help`/`--help`/`-h` 등)·flag 이름·positional 인자를 보존해야 해요 — 추가·삭제·개명 없는 순수 구조 마이그레이션. **예외**: `USAGE` 와 실제 dispatch 간 명백한 버그성 불일치(오타 flag, 문서 누락 명령 노출 등)는 정정할 수 있지만, 각 정정은 (a) 명세/플랜에 별도 명시·추적되고 (b) 기존 외부 관측 동작 parity 를 깨지 않아야 해요.

### Key Entities *(CLI 구조)*

- **Command set**: 약 50개 subcommand. 분류 — hook 진입점(fail-open 필수), 사용자 직접 호출(데이터/변경), 분석·유지보수, hidden(문서 미노출). 각 명령은 이름·별칭·flag·positional·stdin 사용 여부·exit code 의미를 가져요.
- **Exit-code taxonomy**: 바이너리 **자체 process exit** 은 0(성공)·64(usage 오류)·65(데이터/입력 오류) — 테스트가 `status.code()` 로 직접 잠가요. 70(내부 데이터 오류)·124(timeout)·127(spawn 실패)는 **process exit 이 아니라** shelled-out `axhub` CLI 결과·data-layer 결과를 JSON 으로 *보고*하는 값(보존 대상이나 분류축이 다름 — data-model.md §3 참조).
- **출력 채널 계약**: stdout = 기계 판독용 결과/JSON, stderr = 진단·에러·hook 부가 메시지. hook JSON 형태는 Claude Code 가 소비하므로 불변.
- **Parity oracle**: `crates/axhub-helpers/tests/` 의 기존 통합 테스트 모음(`cli_e2e.rs`, `hook_safety_cli.rs`, `version_quiet_test.rs`, `data_layer_cli.rs`, `bootstrap_*`, `deploy_prep_test.rs` 등)이 회귀 게이트 역할을 해요.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001 (회귀 게이트)**: 기존 axhub-helpers 통합 테스트 모음이, 의도적으로 갱신하는 **top-level usage-error 문구** assert(약 1–2곳, 예: `cli_e2e.rs` 의 `"unknown subcommand"` 단언) 와 별도 추적된 버그성 정정(FR-013) 외에는 **수정 없이** 모두 통과해요. per-command 한국어 help/error 문구를 잠그는 assert 는 보존(FR-006a)되므로 변경하지 않아요.
- **SC-002 (외부 계약 무수정)**: `hooks/hooks.json`, `hooks/*.sh`, `hooks/*.ps1`, SKILL invocation 에 **0건**의 편집으로 전부 기존과 동일하게 동작해요.
- **SC-003 (fail-open 검증)**: 무인자/unknown-ignore hook 명령(`session-start`, `prompt-route`, `preauth-check`, `commit-gate`, `tdd-inject`, `classify-exit`, `test-classifier`)은 잘못된/알 수 없는 인자에서 **exit 0** 으로 끝나요(exit 2 도 64 도 아님). flag-bearing hook(`state-update`)은 **유효 hook-flag 경로**(`--edit-event` 등) 실패 시 exit 0 이되, malformed/비-hook 입력(`--bogus`·무인자)은 기존 **exit 64 보존**(FR-001 parity guard, data-model §4 참조).
- **SC-004 (손수 파싱 제거)**: subcommand dispatch 진입점에서 per-command `while i < args.len()` flag 파싱 루프 개수가 **0** 이에요.
- **SC-005 (version --quiet 보존)**: `version --quiet` 와 `--version --quiet` 가 인자 순서와 무관하게 빈 stdout+stderr+exit 0 을 내요.
- **SC-006 (usage exit code)**: 알 수 없는 subcommand·필수 인자 누락·subcommand 미지정이 모두 **exit 64** 로 끝나요.
- **SC-007 (빌드 건전성)**: `cargo build`·`cargo clippy --all-targets -- -D warnings`·`cargo test` 가 깨끗하게 통과하고, `Cargo.toml` 에 새 런타임 의존성이 추가되지 않아요.
- **SC-008 (단일 source of truth)**: 명령에 flag 하나를 추가할 때 한 군데(해당 명령 정의)만 건드리면 돼요 — 기존처럼 파싱 루프 + `USAGE` 상수 + dispatch 세 곳을 동기화할 필요가 없어요.

## Assumptions

- 이미 workspace dependency 인 `clap` 을 derive API 로 채택해요. 새 런타임 의존성은 추가하지 않아요.
- 소스에는 현재 clap 사용이 없어요("제로에서 채택"). lib 모듈 중 `&[String]` 을 받는 것들(`resolve.rs`, `sync.rs`, `snippet.rs`, `deploy_prep.rs`, `bootstrap.rs`)을 clap 파싱 구조체로 옮길지, 시그니처를 유지하고 진입점에서만 clap 으로 파싱할지는 `/plan` 단계의 설계 결정이에요.
- 약 50개 subcommand 전체를 옮기는 게 최종 상태예요. **순서(incremental coexistence vs big-bang)는 `/plan` 단계에서 결정**하고, 이 명세는 위험도순 P1→P2→P3 우선순위만 제시해요.
- 기존 통합 테스트 모음이 parity oracle 이에요. 동작 동일성은 테스트 통과로 검증해요.
- 사용자 결정에 따라, 기계/외부 관측 동작(exit code·JSON·`version --quiet`)은 1바이트도 안 바꿔요. **top-level `--help` 레이아웃과 종합 usage-error 문구만** clap 자동 생성 형태(영어)를 허용하고(FR-006), per-command 큐레이션 한국어 help/error 는 보존해요(FR-006a). usage-error 의 exit code 는 여전히 64 로 remap 해요.
- 명령 set 은 동결이에요 — 이름·별칭·flag·positional 추가/삭제/개명 없음(FR-013). 단 `USAGE`↔dispatch 간 명백한 버그성 불일치 정정은 별도 추적 + parity 무영향 조건으로 허용하고, 발견 시 `/plan` 단계에서 명시적으로 목록화해요.
- 현재 edition/toolchain(workspace 설정)을 그대로 따르고 MSRV 를 올리지 않아요.

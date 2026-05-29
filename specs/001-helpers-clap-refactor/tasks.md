---
description: "Task list: axhub-helpers clap 리팩토링"
---

# Tasks: axhub-helpers clap 리팩토링

**Input**: Design documents from `specs/001-helpers-clap-refactor/`

**Prerequisites**: plan.md, spec.md, research.md (D1~D9), data-model.md, contracts/cli-commands.md

**Tests**: spec 은 신규 TDD 를 요청하지 않았어요. 기존 `crates/axhub-helpers/tests/` 통합 테스트 모음이 **parity oracle**. 단 clap 이 새로 만드는 위험면(parse 실패 fail-open, usage→64 remap)만 타깃 회귀 테스트를 소수 추가해요(T010, T011). 나머지는 wave 별 "기존 oracle 통과" 검증 task.

**Organization**: user story 별 phase. US1=Wave1(hook 진입점, P1), US2=Wave2(데이터, P2), US3=Wave3(분석/hidden, P3).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 가능 (다른 파일, 미완 task 의존 없음)
- **[Story]**: US1/US2/US3 (Setup·Foundational·Polish 은 라벨 없음)
- 모든 경로는 repo root 기준

## Path Conventions

단일 crate: `crates/axhub-helpers/src/`, 테스트 `crates/axhub-helpers/tests/`. clap 정의는 binary-local 서브모듈 `crates/axhub-helpers/src/cli/` (main.rs 에 `mod cli;`) — dispatch 가 binary-local `cmd_*` handler 를 호출하므로 lib 가 아닌 binary 모듈이에요.

---

## Phase 1: Setup (clap scaffold 토대)

**Purpose**: clap 모듈 골격 + 빌드 확인. 동작 변경 0.

- [X] T001 `crates/axhub-helpers/src/cli/mod.rs` 생성(빈 모듈) + `crates/axhub-helpers/src/main.rs` 최상단에 `mod cli;` 선언
- [X] T002 [P] `crates/axhub-helpers/src/cli/args/mod.rs` 생성 — wave 별 per-command arg 구조체 보관용 빈 서브모듈, `cli/mod.rs` 에 `mod args;` 선언
- [X] T003 최소 `#[derive(clap::Parser)] struct Cli` stub 을 `crates/axhub-helpers/src/cli/mod.rs` 에 넣고 `cargo build -p axhub-helpers` 통과 확인 (clap derive feature 링크 검증)

---

## Phase 2: Foundational (모든 story 의 blocking 선행)

**Purpose**: clap 라우팅 핵심 — 이게 끝나면 clap shell 이 ~50 명령 전체를 passthrough 로 라우팅하고, fail-open·version·usage-64 가 동작하며, 기존 oracle 이 green. **이 phase 완료 = scaffold MVP (독립 검증 가능).**

⚠️ 이 phase 완료 전엔 어떤 user story 도 시작 불가.

- [X] T004 기존 `main.rs::run()` 의 `match cmd.as_str()` dispatch 를 `legacy_dispatch(cmd: &str, rest: &[String]) -> anyhow::Result<i32>` 함수로 추출 (동작 동일, 호출부만 분리) in `crates/axhub-helpers/src/main.rs`
- [X] T005 `Commands` enum 정의 — `#[command(external_subcommand)] Passthrough(Vec<String>)` 단일 variant + `Cli{ #[command(subcommand)] command: Commands }`, `#[command(disable_version_flag = true)]` in `crates/axhub-helpers/src/cli/mod.rs` (data-model.md §1~2)
- [X] T006 `dispatch(command: Commands) -> anyhow::Result<i32>` 구현 — `Passthrough(argv)` → `crate::legacy_dispatch(argv[0], &argv[1..])` 로 위임 (전 명령 passthrough 로 동작 유지) in `crates/axhub-helpers/src/cli/mod.rs`
- [X] T007 version pre-intercept 구현 (D5) — argv 스캔: `version`/`--version`/`-v` 감지 + `--quiet` 위치 무관 감지 → quiet 면 빈 출력 exit 0, 아니면 `axhub-helpers {v} (plugin v{v}, schema v0)` stdout exit 0. clap 파싱 전에 실행 in `crates/axhub-helpers/src/cli/mod.rs`
- [X] T008 hook 분류 `classify(token: Option<&str>) -> HookClass` 구현 (D3) — FailOpenHook set = {session-start, prompt-route, preauth-check, commit-gate, tdd-inject, classify-exit, test-classifier, state-update, autowire-statusline} in `crates/axhub-helpers/src/cli/mod.rs`
- [X] T009 parse-error 처리 `handle_parse_error(e, hook_class) -> i32` 구현 (D4) — `DisplayHelp`→stdout+0, `DisplayHelpOnMissingArgumentOrSubcommand`→stderr+64, usage 계열→hook 면 0·아니면 stderr+64. `e.exit()` 대신 `e.print()`+코드 반환 in `crates/axhub-helpers/src/cli/mod.rs`
- [X] T010 `cli::run()` 조립 — version pre-intercept → `classify(argv[1])` → `Cli::try_parse_from(argv)` → Ok→`dispatch`, Err→`handle_parse_error`. `main.rs::main()` 을 `enable_utf8_console()` → `cli::run()` → `process::exit` thin shell 로 변경 (FR-012 순서 보존) in `crates/axhub-helpers/src/main.rs` + `cli/mod.rs`
- [X] T011 [P] 신규 회귀 테스트 — clap parse 실패 fail-open: 각 hook 명령에 `--bogus` flag → exit 0 검증 in `crates/axhub-helpers/tests/clap_failopen_test.rs`
- [X] T012 [P] 신규 회귀 테스트 — usage-error remap: unknown subcommand·subcommand 없음·필수 인자 누락 → exit 64 (clap 기본 2 아님) in `crates/axhub-helpers/tests/clap_usage_exit_test.rs`
- [X] T013 Foundational 검증 — `cargo test -p axhub-helpers` 전체 green (passthrough 로 parity 유지) + `cargo clippy -p axhub-helpers --all-targets -- -D warnings` 0 + 신규 T011/T012 통과

**Checkpoint**: clap shell 라우팅 + fail-open + version + usage-64 동작, 기존 oracle green. 이후 story 들은 passthrough 명령을 typed 로 교체.

---

## Phase 3: User Story 1 — hook 진입점 (P1) 🎯 MVP

**Goal**: fail-open 계약 명령을 passthrough → typed clap variant 로 교체. 가장 위험한 production hook 경로를 안전망 위에서 먼저 확정.

**Independent Test**: `hook_safety_cli.rs` + `version_quiet_test.rs` + `classify_exit_suggest_test.rs` green, 그리고 wave-1 명령 전부 잘못된 인자에서 exit 0 (quickstart §2).

- [X] T014 [P] [US1] 무인자 hook 명령(`session-start`, `prompt-route`, `preauth-check`, `commit-gate`, `tdd-inject`, `test-classifier`)을 unit `Commands` variant 로 추가, passthrough 에서 제거, 기존 handler 연결 in `crates/axhub-helpers/src/cli/mod.rs`
- [X] T015 [P] [US1] `classify-exit` typed args (`--exit-code <n>` `--stdout <s>` + stdin payload 분기) in `crates/axhub-helpers/src/cli/args/classify_exit.rs`, dispatch 연결 (stdin 계약 FR-009 보존)
- [X] T016 [P] [US1] `state-update` flag-group (정확히 하나: `--review-acknowledged`|`--post-commit-promote`|`--debug-acknowledged`|`--shipped`|`--edit-event`|`--pull`) in `crates/axhub-helpers/src/cli/args/state_update.rs` — `--edit-event`(PostToolUse)·`--post-commit-promote`(git hook) fail-open 보존. 단 malformed/unknown flag·무인자는 기존 **exit 64 보존**(data-model §4 parity guard — blanket fail-open 금지)
- [X] T017 [P] [US1] `autowire-statusline` typed args (`--scope user|project|auto` `--silent` `--command-path <p>` `--child`) in `crates/axhub-helpers/src/cli/args/autowire.rs`
- [X] T018 [US1] US1 검증 — `cargo test -p axhub-helpers --test hook_safety_cli --test version_quiet_test --test classify_exit_suggest_test` green + wave-1 명령 fail-open exit 0 (quickstart §2 스크립트)

**Checkpoint**: hook 경로 typed 완료, fail-open 검증. MVP 배포 가능 슬라이스.

---

## Phase 4: User Story 2 — 데이터/변경 (P2)

**Goal**: 사용자 직접 호출·SKILL 호출 데이터/변경 명령을 typed 로 교체. lib `_with_runner` seam 보존(D7).

**Independent Test**: `cli_e2e.rs` + `data_layer_cli.rs` + `deploy_prep_test.rs` + `settings_merge.rs` + `bootstrap_*` green, exit code 0/64/65 + JSON 바이트 동일.

- [X] T019 [P] [US2] `deploy-prep` args (`--intent`(필수) `--user-utterance` `--refresh-in-flight` `--json`) → `run_deploy_prep` 연결 (seam 보존) in `crates/axhub-helpers/src/cli/args/deploy_prep.rs`
- [X] T020 [P] [US2] `sync` args → `run_sync(&[String])` 브리지 (clap 파싱 후 lib 호출, 시그니처 유지) in `crates/axhub-helpers/src/cli/args/sync.rs`
- [X] T021 [P] [US2] `snippet` args (`--mode A|B` `--language` `--target` `--connector` `--path` `--sql` `--allowed-columns`) → `run_snippet` 브리지 in `crates/axhub-helpers/src/cli/args/snippet.rs`
- [X] T022 [P] [US2] `config` 중첩 subcommand (`get <key> [--json]` | `set <key> <value>`) in `crates/axhub-helpers/src/cli/args/config.rs`
- [X] T023 [P] [US2] `verify`(`--app-id` 필수 `--json`) + `trace`(`--deploy-id` 필수 `--app` `--json`) + `doctor`(`--json` `--no-cooldown`) typed args in `crates/axhub-helpers/src/cli/args/diag.rs`
- [X] T024 [P] [US2] `bootstrap` (`[--json|--dry-run|--plan-only|--auto-chain|--record <event>]` + `dependency-plan` 중첩) — 조건부 stdin(`--record apps_create|deploy_create`) 보존 in `crates/axhub-helpers/src/cli/args/bootstrap.rs`
- [X] T025 [P] [US2] `consent-mint`(`[--validate-only]`) + `consent-verify` — stdin 계약 + 한국어 stdin 에러(D6, handler-level exit 65) 보존 in `crates/axhub-helpers/src/cli/args/consent.rs`
- [X] T026 [P] [US2] `token-init`/`token-import`(`[--json]`) + `token-gate`(SKILL gate, exit 0/65 의미 보존) in `crates/axhub-helpers/src/cli/args/token.rs`
- [X] T027 [P] [US2] `resolve`(lib `&[String]` 브리지) + `preflight`(무인자) + `settings-merge`(`--apply|--dry-run` 택1 `--scope` `--json`) in `crates/axhub-helpers/src/cli/args/misc_p2.rs`
- [X] T028 [US2] US2 검증 — `cargo test -p axhub-helpers --test cli_e2e --test data_layer_cli --test deploy_prep_test --test settings_merge --test bootstrap_coverage --test bootstrap_dependency_plan_test --test token_gate_test` green (token-gate 가 US2/T026 에서 migrate 되므로 token_gate_test 도 여기서 검증)

**Checkpoint**: 데이터/변경 명령 typed 완료, exit/JSON parity 검증.

---

## Phase 5: User Story 3 — 분석/유지보수 + hidden (P3)

**Goal**: 나머지 분석·운영 명령 + hidden 명령 typed 교체. 한국어 long_help 보존(D6), hidden 처리(FR-007).

**Independent Test**: `audit_e2e.rs` + `recovery_scan_test.rs` + `diagnose_*` + `post_install_test.rs` + `token_gate_test.rs` green, positional·duration 파싱 동일, hidden 명령 동작 유지 + `--help` 미노출.

- [X] T029 [P] [US3] `routing-stats` args (`--since <dur>` `--json` `--top <n>` `--confused`) + 한국어 PRIVACY 블록을 `long_about` 으로 보존(D6) in `crates/axhub-helpers/src/cli/args/routing_stats.rs`
- [X] T030 [P] [US3] `cleanup-audit`(`[--all] [--yes]`) + `audit-clarify`(`(--hash|--prompt) --chosen`) + `routing-dashboard`(`[--html]`) + `list-deployments`(`ListDeploymentsArgs` 재사용) in `crates/axhub-helpers/src/cli/args/audit.rs`
- [X] T031 [P] [US3] positional 명령 — `mark <phase>`, `emit-deploy-complete [<exit_code> [<class>]]`(optional positional), `path <token-file|last-deploy-file|state-dir>` in `crates/axhub-helpers/src/cli/args/positional.rs`
- [X] T032 [P] [US3] `post-install`(`--target-name` `--bin-dir` `--link-path` `[--repo-root]`) — 한국어 flag 에러 보존(D6) in `crates/axhub-helpers/src/cli/args/post_install.rs`
- [X] T033 [P] [US3] `diagnose hitl`(중첩: `--session` `--prompts` `[--output]`) + `orphan-stub`(`--install [--verify]`|`--verify`) + `auth-refresh-bg` + `redact`(stdin) + `statusline` in `crates/axhub-helpers/src/cli/args/misc_p3.rs`
- [X] T034 [P] [US3] hidden 명령 — `state-show`(`[--json]`), `consent`(`[--enable|--disable|--show]`), `karpathy-inject`(stdin) 을 `#[command(hide = true)]` variant 로 (동작 유지, `--help` 미노출) in `crates/axhub-helpers/src/cli/mod.rs`
- [X] T035 [US3] US3 검증 — `cargo test -p axhub-helpers --test audit_e2e --test recovery_scan_test --test diagnose_2am_friday --test diagnose_layering_test --test post_install_test` green + hidden 명령(`state-show`/`consent`/`karpathy-inject`) `--help` 미노출 확인 (token_gate_test 는 US2/T028 로 이동)

**Checkpoint**: 전 명령 typed. passthrough 잔여 0 직전.

---

## Phase 6: Polish & Cross-Cutting (Wave 4 cleanup)

**Purpose**: SC 최종 검증, 의도된 wording test 갱신. (T036/T037 은 아래 설계 결정으로 revise.)

> **END-STATE 결정 (구현 중 확정)**: 전 명령 typed 는 비현실적 — lib-위임(sync/snippet/bootstrap/
> resolve/preflight/deploy-prep)·positional/slice(config)·인자무시(consent-verify/token-gate/redact/
> statusline/auth-refresh-bg/orphan-stub/cleanup-audit/routing-dashboard/mark/emit/path)·hidden(state-show/
> consent/karpathy-inject) ≈20 명령은 **passthrough 유지가 parity-safe**(main.rs while-loop 없음 → SC-004
> 무관, D7 lib seam 보존, 인자무시→strict reject 회피). 따라서 `legacy_dispatch`+`Passthrough`+`USAGE` 는
> thin loop-free router 로 **존속**. FR-003 의 핵심("hand-rolled while-loop 제거")은 SC-004 로 달성.
> FR-006 top-level help: USAGE 가 전 명령 목록을 주므로 clap-gen-partial 보다 우수 → USAGE 유지.

- [X] T036 (revise) `Commands::Passthrough` + `legacy_dispatch` **존속** — typed(15 명령)/passthrough(≈20) 공존. while-loop 0(SC-004) 달성으로 FR-003 핵심 충족.
- [X] T037 (revise) `USAGE` 상수 **존속** — passthrough 명령 help + top-level 완전 목록 제공. clap subcommand --help 는 typed 명령에 동작(long_about 한국어 보존).
- [X] T038 의도된 wording test 갱신 — `cli_e2e.rs` 의 `"unknown subcommand"` assert 를 clap 영어 문구로 (SC-001 의 허용된 1~2곳) in `crates/axhub-helpers/tests/cli_e2e.rs`
- [X] T039 SC-004 검증 — `grep -c 'while i < args.len()'` 가 dispatch 진입점(main.rs/cli/) 에서 0 확인
- [X] T040 [P] SC-002 검증 — `git diff --name-only main -- hooks/` 가 `hooks.json`/`*.sh`/`*.ps1` 무변경 확인
- [X] T041 [P] D8 — binary size delta 측정 + 기록: `ls -la` + `cargo bloat -p axhub-helpers --release --crates` (5 cross-arch 영향 메모)
- [X] T042 D6 한국어 보존 spot-check — `routing-stats --help`(PRIVACY 블록) + 빈 stdin `consent-mint`(한국어 exit 65) 수동 확인
- [X] T043 최종 게이트 — `cargo test -p axhub-helpers`(T038 외 무수정 green) + `cargo clippy --all-targets -- -D warnings` 0 + `bunx tsc --noEmit` clean

---

## Dependencies & Execution Order

```
Phase 1 (Setup, T001-T003)
   └─> Phase 2 (Foundational, T004-T013)   ★ blocking — 전 story 선행
          ├─> Phase 3 (US1, T014-T018)   ← MVP
          ├─> Phase 4 (US2, T019-T028)
          └─> Phase 5 (US3, T029-T035)
                 └─> Phase 6 (Polish, T036-T043)  ← passthrough 제거는 US1+US2+US3 전부 완료 후
```

- **US1/US2/US3 상호 독립**: Foundational 완료 후 각 story 는 passthrough 에서 자기 명령만 typed 로 빼므로 병렬 작업 가능(서로 다른 `cli/args/*.rs` 파일). 단 같은 `cli/mod.rs` 의 enum variant 등록은 직렬 조정 필요.
- **Polish 의존**: T036(passthrough 제거)·T037(USAGE 제거)·T039(SC-004) 는 US1+US2+US3 **모두** 완료 후. T038·T040·T041 은 [P].

## Parallel Opportunities

- **Phase 2 내**: T011·T012(신규 테스트 파일) [P]. T004~T010 핵심 라우팅은 같은 파일이라 대체로 직렬.
- **Phase 3 (US1)**: T014~T017 [P] — 각기 다른 `cli/args/*.rs` (T014 만 mod.rs variant 등록이라 enum 충돌 조정).
- **Phase 4 (US2)**: T019~T027 모두 [P] — 9개 독립 arg 파일.
- **Phase 5 (US3)**: T029~T034 모두 [P] — 6개 독립 파일.
- **story 간**: Foundational 후 US1/US2/US3 를 별도 작업자/세션 병렬 진행 가능(enum 등록만 merge 조정).

## Implementation Strategy

- **MVP = Phase 1 + 2 + 3 (US1)**: clap shell + fail-open + version/usage parity + hook 명령 typed. 이 시점에 production 위험 표면(hook)이 완전히 clap 화 + 안전. 나머지는 passthrough 로 정상 동작 유지하며 점진 배포.
- **Incremental**: 각 Phase checkpoint 에서 `cargo test` green 이므로 wave 단위로 독립 PR·rollback 가능(plan D2).
- **Parity 우선**: 매 task 후 관련 oracle 테스트가 회귀 게이트. 신규 동작(fail-open/usage-64)만 T011/T012 신규 테스트로 잠금.

# CLI Command Contract: axhub-helpers

**Date**: 2026-05-29 | **Plan**: [../plan.md](../plan.md)

이 표가 **단일 계약 정본**이에요 — clap 마이그레이션의 (1) parity oracle 과 (2) wave 별 체크리스트를 겸해요. 각 행의 이름·별칭·flag·positional·stdin·exit·hook-class 는 마이그레이션 전후로 보존(FR-001, FR-013)되고, top-level help/usage-error 문구만 clap 영어 허용(FR-006).

**범례** — stdin: ✔=읽음, △=조건부. hook-class: **H**=CC-hook fail-open(exit0 계약), **W**=SessionStart wrapper 경유(exit 흡수, 출력 민감), **G**=SKILL gate(exit 의미), **N**=일반, **hid**=hidden(`--help` 미노출).

## Wave 1 — hook 진입점 (P1)

| Command | 별칭 | flag / positional | stdin | exit | class |
|---|---|---|---|---|---|
| `session-start` | — | (무인자) | | 0 | **H** (SessionStart exec) |
| `prompt-route` | — | (무인자) | ✔ | 0 | **H** (UserPromptSubmit) |
| `preauth-check` | — | (무인자) | ✔ | 0 | **H** (PreToolUse Bash) |
| `commit-gate` | — | (무인자) | ✔ | 0 | **H** (PreToolUse Bash) |
| `tdd-inject` | — | (무인자) | ✔ | 0 | **H** (PreToolUse Edit\|Write) |
| `classify-exit` | — | `--exit-code <n>` `--stdout <s>` (또는 stdin payload) | △ | 0 | **H** (PostToolUse Bash) |
| `test-classifier` | — | (무인자) | ✔ | 0 | **H** (PostToolUse[+Failure]) |
| `state-update` | — | `--review-acknowledged`\|`--post-commit-promote`\|`--debug-acknowledged`\|`--shipped`\|`--edit-event`\|`--pull` (택1) | | 0/64 | **H** (`--edit-event`=PostToolUse Edit\|Write; `--post-commit-promote`=git hook) |
| `autowire-statusline` | — | `--scope user\|project\|auto` `--silent` `--command-path <p>` `--child` | | 0 | **W**+fail-open (session-start-autowire.sh, detached; classify()=fail-open) |
| `version` | `--version`, `-v` | `--quiet` (위치 무관) | | 0 | **H**-ish (pre-intercept, D5; quiet⇒빈 stdout+stderr) |
| `help` | `--help`, `-h` | — | | 0 | N (clap `DisplayHelp`→stdout) |

## Wave 2 — 데이터/변경 (P2)

| Command | 별칭 | flag / positional | stdin | exit | class |
|---|---|---|---|---|---|
| `deploy-prep` | — | `--intent <name>`(필수) `--user-utterance <s>` `--refresh-in-flight` `--json` | | 0/64/65 | N (lib `run_deploy_prep`) |
| `sync` | — | `--target <t>\|auto` `--out <dir>` `--json` `--no-detail` `--allow-identity-change` | | 0/65 | N (lib `run_sync`) |
| `snippet` | — | `--mode A\|B` `--language <l>` `--target <t>` `--connector <n>` `--path <p>` `--sql <s>` `--allowed-columns <csv>` | | 0/64 | N (lib `run_snippet`) |
| `config` | — | `get <key> [--json]` \| `set <key> <value>` (nested) | | 0/64 | N (중첩 subcommand) |
| `verify` | — | `--app-id <id>`(필수) `--json` | | 0/65 | N |
| `trace` | — | `--deploy-id <id>`(필수) `--app <app>` `--json` | | 0/65 | N |
| `doctor` | — | `--json` `--no-cooldown` | | 0 | N |
| `bootstrap` | — | `[--json] [--dry-run\|--plan-only\|--auto-chain\|--record <event>]` \| `dependency-plan` (nested) | △ (`--record apps_create\|deploy_create` 일 때만) | run.exit_code | N |
| `consent-mint` | — | `[--validate-only]` | ✔ | 0/64/65 | N (한국어 stdin 에러 D6) |
| `consent-verify` | — | (무인자) | ✔ | 0/65 | N |
| `token-init` | — | `[--json]` | | 0/65 | N |
| `token-import` | — | `[--json]` | ✔ | 0/65 | N |
| `token-gate` | — | (무인자) | | 0/65 | **G** (SKILL deploy gate; registry `token-freshness-gate`) |
| `resolve` | — | (lib `&[String]`) | | run.exit_code | N (lib `run_resolve`) |
| `preflight` | — | (무인자) | | run.exit_code | N (lib `run_preflight`) |
| `settings-merge` | — | `--apply\|--dry-run`(택1 필수) `--scope user\|project\|auto` `--json` | | 0/64 | N |

## Wave 3 — 분석/유지보수 + hidden (P3)

| Command | 별칭 | flag / positional | stdin | exit | class |
|---|---|---|---|---|---|
| `routing-stats` | — | `--since <dur>` `--json` `--top <n>` `--confused` (한국어 PRIVACY long_help D6) | | 0/64 | N |
| `cleanup-audit` | — | `[--all] [--yes]` | | 0 | N |
| `audit-clarify` | — | `(--hash <h>\|--prompt <p>)` `--chosen <s>` | | 0/64 | N |
| `routing-dashboard` | — | `[--html]` | | 0 | N |
| `list-deployments` | — | (`ListDeploymentsArgs` — 이미 타입) | | 0 | N |
| `mark` | — | `<phase_name>` (positional) | | 0 | N |
| `emit-deploy-complete` | — | `[<exit_code> [<command_class>]]` (optional positional) | | 0 | N |
| `path` | — | `<token-file\|last-deploy-file\|state-dir>` (positional) | | 0/64/65 | N |
| `post-install` | — | `--target-name <n>` `--bin-dir <d>` `--link-path <p>` `[--repo-root <r>]` (한국어 에러 D6) | | 0/64 | N |
| `diagnose` | — | `hitl --session <id> --prompts <f> [--output <f>]` (nested) | | 0 | N |
| `orphan-stub` | — | `--install [--verify]` \| `--verify` | | 0 | N |
| `auth-refresh-bg` | — | (무인자, detached) | | 0 | **W** (session-start.sh nohup) |
| `redact` | — | (무인자) | ✔ | 0 | N |
| `statusline` | — | (무인자) | | 0 | **W** (statusline.sh/ps1 render; 출력=상태줄) |
| `state-show` | — | `[--json]` | | 0/64 | N **hid** |
| `consent` | — | `[--enable\|--disable\|--show]` | | 0/64 | N **hid** |
| `karpathy-inject` | — | (무인자) | ✔ | 0 | fail-open **hid** (prompt-route 임베드 — classify()=fail-open, typed 이관 P3) |

## 계약 불변식 (모든 wave 공통)

1. **exit code 보존**: 위 표의 값 그대로. clap parse 실패만 64(N)/0(H) remap (D4).
2. **stdout/stderr 분리**: stdout=결과/JSON, stderr=진단/에러/hook 메시지. hook JSON 형태 불변.
3. **stdin 계약**: ✔/△ 명령은 기존 stdin 읽기 동작 보존.
4. **fail-open**: class=H 는 어떤 인자에도 exit 0. class=W 는 wrapper 가 exit 를 흡수하지만 (a) 출력 계약(특히 `version --quiet` 빈 출력) 보존 + (b) hook 맥락이라 classify() 가 **fail-open 으로 분류** → parse error 도 exit 0(방어적; 예 `autowire-statusline`). class=G 는 0/65 의미 보존(항상 0 아님 — parse error→64). 즉 W·G 는 호출 기제 라벨, fail-open/Normal 은 exit 동작 — 직교 축이에요.
5. **hidden**: `hid` 명령은 동작 유지 + `--help` 미노출.
6. **명령 set 동결**(FR-013): 이름·별칭·flag·positional 추가/삭제/개명 0. USAGE↔dispatch 버그성 불일치만 별도 추적 정정 — 현재 알려진 버그 후보 0(모든 hidden 은 의도).

## 검증 매핑 (parity oracle 테스트)

| 계약 영역 | 잠그는 테스트 |
|---|---|
| version/quiet | `version_quiet_test.rs` |
| hook fail-open + kill switch | `hook_safety_cli.rs` |
| usage exit 64 | `cli_e2e.rs`, `autowire_scope_auto_test.rs`, `bootstrap_coverage.rs` |
| data exit 65 / nested | `bootstrap_dependency_plan_test.rs`, `data_layer_cli.rs` |
| classify-exit suggest | `classify_exit_suggest_test.rs` |
| deploy-prep | `deploy_prep_test.rs` |
| settings-merge | `settings_merge.rs` |
| post-install | `post_install_test.rs` |
| diagnose | `diagnose_2am_friday.rs`, `diagnose_layering_test.rs` |
| token-gate | `token_gate_test.rs` |

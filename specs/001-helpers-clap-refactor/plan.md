# Implementation Plan: axhub-helpers clap 리팩토링

**Branch**: `001-helpers-clap-refactor` | **Date**: 2026-05-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-helpers-clap-refactor/spec.md`

## Summary

`axhub-helpers` 바이너리의 진입점(`crates/axhub-helpers/src/main.rs`, 약 3,300줄)을 손수 작성한 `match cmd.as_str()` dispatch + per-command `while i < args.len()` flag 루프에서 **clap 4 derive** 기반 declarative 명령 정의로 옮겨요. 외부 관측 동작(exit code 0/64/65/70/124/127, stdout/stderr 분리, JSON 바이트, `version --quiet` 어느 순서든 빈 출력)은 1바이트도 안 바꾸고, top-level `--help` 레이아웃·종합 usage-error 문구만 clap 자동 생성(영어)을 허용해요. per-command 큐레이션 한국어 help/error 는 clap `long_about`/handler-level 메시지로 보존해요.

기술적 핵심은 세 가지예요: (1) **fail-open 보존** — Claude Code hook 이 호출하는 subcommand 는 clap 파싱 실패에도 exit 0, (2) **usage-error exit code remap** — clap 기본 exit 2 를 64 로, (3) **version pre-intercept** — clap 이 argv 를 보기 전에 version 의도를 가로채요. 마이그레이션은 위험도순 P1(hook)→P2(데이터)→P3(분석) wave 로 진행하고, 각 wave 사이 바이너리는 항상 빌드·테스트 green 을 유지해요.

## Technical Context

**Language/Version**: Rust, edition **2021**, MSRV **1.83** (workspace 고정 — 올리지 않아요)

**Primary Dependencies**: `clap = { version = "4", features = ["derive"] }` — 이미 workspace 에 선언돼 있지만 소스 미사용(현재 미링크). 이번에 derive API 로 채택. 새 런타임 의존성 추가 없음.

**Storage**: N/A — CLI adapter. 로컬 state 파일(token, audit log, quality state) 읽기/쓰기는 기존 lib 모듈이 담당, 이번 작업 범위 밖.

**Testing**: `cargo test` — `crates/axhub-helpers/tests/` 의 기존 통합 테스트 모음(약 28개 파일)이 **parity oracle**. 추가로 `cargo clippy --all-targets -- -D warnings`, `bunx tsc`(인접 TS hook 영향 없음 확인).

**Target Platform**: macOS(arm64/x64), Linux(arm64/x64), Windows(x64) — release.yml 이 빌드하는 5개 cross-arch 바이너리. Windows UTF-8 console codepage 초기화 보존 필요.

**Project Type**: 단일 CLI 바이너리(`axhub-helpers`) — cargo workspace 내 1개 crate.

**Performance Goals**: 수동 파싱 대비 회귀 없음. clap derive 파싱 비용은 프로세스 spawn 대비 무시 가능(µs 단위). hook hot-path startup 비용 불변. **binary size delta 는 측정 + 보고 항목**(clap 링크로 증가 예상, research.md 참조).

**Constraints**:
- 외부 관측 동작 byte-identical (exit code·stdout/stderr 분리·JSON)
- CC-hook 호출 subcommand 는 fail-open exit 0
- MSRV 1.83 / edition 2021 유지 (let-chains·1.88 기능 금지)
- 새 런타임 dep 0
- `hooks/hooks.json`·`hooks/*.sh`·`hooks/*.ps1`·SKILL invocation 무수정
- lib 모듈의 `_with_runner` test seam 보존

**Scale/Scope**: ~50 subcommand, main.rs ~3,300줄, ~28 통합 테스트 파일. 중첩 명령(`config get|set`, `bootstrap dependency-plan`, `diagnose hitl`), positional(`path <kind>`, `mark <phase>`, `emit-deploy-complete [exit_code [class]]`), hidden 명령(`state-show`, `consent`) 포함.

## Constitution Check

*GATE: Phase 0 research 전 통과 필수. Phase 1 design 후 재확인.*

**상태: 프로젝트 constitution(`.specify/memory/constitution.md`)은 미작성 템플릿(placeholder 그대로)이에요. 비준된 원칙이 없어서 위반할 구체 gate 가 없어요.** 따라서 Constitution Check 는 공허하게 PASS 하되, 이 repo 의 문서화된 non-negotiable 룰(CLAUDE.md)을 operative 제약으로 대체 적용해요:

| 운영 gate (CLAUDE.md 출처) | 적용 | 상태 |
|---|---|---|
| Hook fail-open: 모든 hook 진입점 exit 0 (docs/HOOKS.md, hook-safety ADR §10.6) | FR-002 가 직접 인코딩. clap 파싱 실패 가로채기 설계 | ✅ PASS |
| `hook_safety::is_hook_disabled` kill switch 보존 | dispatch 가 기존 분기 우회 안 함 (FR-010) | ✅ PASS |
| 외부 동작 parity (test/hooks.json/SKILL 계약) | FR-001, SC-001~006 | ✅ PASS |
| Surgical change / drift-aversion (CLAUDE.md §3) | 명령 set 동결 FR-013, scope wave 분리 | ✅ PASS |
| Rust CLI 관례: clap derive, stdout=data/stderr=logs, ExitCode (rules/rust/cli.md) | 설계가 정확히 이 관례 채택 | ✅ PASS |
| 새 의존성 추가 금지 | clap 이미 존재, 0 신규 (FR-011, SC-007) | ✅ PASS |

> Release workflow / SKILL scaffold / 톤 lint 룰은 이 feature 범위 밖(소스 바이너리 리팩토링, SKILL·릴리스 산출물 미변경)이라 N/A.

**Complexity 위반 없음** — 아래 Complexity Tracking 비움.

## Project Structure

### Documentation (this feature)

```text
specs/001-helpers-clap-refactor/
├── plan.md              # 이 파일 (/speckit-plan 출력)
├── research.md          # Phase 0 — 설계 결정 9건 (derive/sequencing/fail-open/version/error-kind/한국어 help/lib 모듈/binary size/MSRV)
├── data-model.md        # Phase 1 — 내부 타입 (Cli parser, Commands enum, ExitCode, HookClass set)
├── contracts/
│   └── cli-commands.md  # Phase 1 — subcommand 계약 표 (= parity oracle + wave 체크리스트)
├── quickstart.md        # Phase 1 — 빌드/테스트/parity 검증 절차
├── checklists/
│   └── requirements.md  # spec 품질 체크리스트 (/speckit-specify 출력)
└── tasks.md             # Phase 2 — /speckit-tasks 출력 (이 명령서 생성 안 함)
```

### Source Code (repository root)

기존 단일 crate 구조 유지. main.rs 를 thin shell 로 만들고 clap 정의를 `cli` 모듈로 분리해요(rules/rust/project-structure.md "keep main.rs thin").

```text
crates/axhub-helpers/
├── src/
│   ├── main.rs              # thin: enable_utf8_console() → cli::run() → process::exit
│   ├── cli/                 # 신규 모듈 — clap 정의 + dispatch
│   │   ├── mod.rs           # Cli(Parser) + Commands(enum) + run() + parse-error 처리 + version pre-intercept + hook 분류
│   │   └── args/            # wave 별 per-command clap args 구조체 (deploy_prep, sync, routing_stats, ...)
│   ├── (기존 lib 모듈 유지)   # resolve.rs, sync.rs, snippet.rs, bootstrap.rs, deploy_prep.rs,
│   │                        # consent.rs, preflight.rs, quality_state.rs, hook_safety.rs, ...
│   │                        #   → `_with_runner` test seam + 시그니처 보존(FR), 파싱만 점진 이관
│   └── lib.rs               # 기존 pub 모듈 선언
└── tests/                   # 무수정 parity oracle (cli_e2e, hook_safety_cli, version_quiet, ...)
                             #   예외: top-level usage-error wording assert 1~2곳 (SC-001)
```

**Structure Decision**: 단일 project(Option 1) 유지. clap 정의를 `crates/axhub-helpers/src/cli/` 신규 모듈에 격리하고 `main.rs` 는 진입 부수동작(UTF-8 console) + `cli::run()` 위임만 남겨요. 기존 cmd_* handler 와 lib 모듈은 transition 중 그대로 호출되고, wave 진행에 따라 각 명령의 flag 파싱이 clap args 구조체로 흡수돼요. 새 crate·새 workspace member 추가 없음.

## Migration Sequencing (wave 개요)

> 상세 결정 근거는 research.md, 명령별 계약은 contracts/cli-commands.md.

- **Wave 0 — clap scaffold**: `Cli`/`Commands` enum 골격 + parse-error 처리 + version pre-intercept + hook 분류 함수. transition 용 `Commands` enum 은 미이관 명령을 raw `Vec<String>` passthrough variant 로 보유해 항상 빌드·green 유지.
- **Wave 1 — hook 진입점 (P1)**: fail-open 계약 명령 우선. 권위 set(hooks.json + SessionStart wrapper 기반):
  `session-start`, `prompt-route`, `preauth-check`, `commit-gate`, `tdd-inject`, `classify-exit`, `test-classifier`, `state-update`(특히 `--edit-event`), + SessionStart wrapper 경유 `autowire-statusline`. `version`/`--version`/`-v` pre-intercept 도 여기.
- **Wave 2 — 데이터/변경 (P2)**: `deploy-prep`, `sync`, `snippet`, `config`, `verify`, `trace`, `doctor`, `bootstrap`(+`dependency-plan`), `consent-mint`/`consent-verify`, `token-init`/`token-import`/`token-gate`, `resolve`, `preflight`, `settings-merge`.
- **Wave 3 — 분석/유지보수 + hidden (P3)**: `routing-stats`, `cleanup-audit`, `audit-clarify`, `routing-dashboard`, `list-deployments`, `mark`, `emit-deploy-complete`, `path`, `post-install`, `diagnose hitl`, `orphan-stub`, `auth-refresh-bg`, `redact`, `statusline`, hidden(`state-show`, `consent`, `karpathy-inject`).
- **Wave 4 — cleanup**: 마지막 raw passthrough variant 제거 → `USAGE` 상수 삭제, `while i<args.len()` 루프 0 확인(SC-004), binary size 측정.

## ⚠️ Spec 보정 (fail-open set)

spec FR-002 의 hook 명령 enumeration 은 코드 inference 였고, hooks.json + SessionStart wrapper 권위 확인 결과 보정이 필요해요. 이 plan 이 정본이에요:

- **추가**: `state-update`(PostToolUse `Edit|Write` → `--edit-event`), `autowire-statusline`(SessionStart wrapper 경유, detached).
- **재분류** (Clarifications 2026-05-29 확정): `karpathy-inject` — 직접 CC hook 은 아니지만(`prompt-route` 임베드) **fail-open 분류**(parse error→exit 0); hidden 이라 typed 이관은 Wave 3 이되 fail-open 분류는 Foundational 에서 보장. `token-gate`(hooks.json 부재 — SKILL deploy Step 3.5 gate, 등록명 `token-freshness-gate`) — **유일한 Normal 예외**: exit 0/65(unauthorized) 의미 보존이라 fail-open(항상 0) 아님, parse error→64.
- 상세 분류는 contracts/cli-commands.md 의 `hook-class` 열 참조. (이 보정은 `/speckit-analyze` 가 spec↔plan inconsistency 로 잡을 항목 — 의도된 정정.)

## Complexity Tracking

> Constitution 위반 없음 — 비움.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (없음) | — | — |

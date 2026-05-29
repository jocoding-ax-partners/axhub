# Phase 0 Research: axhub-helpers clap 리팩토링

**Date**: 2026-05-29 | **Plan**: [plan.md](./plan.md)

모든 NEEDS CLARIFICATION 은 spec + `/speckit-clarify` 에서 해소됐어요. 이 문서는 plan 의 기술 결정 9건을 확정해요.

---

## D1. clap derive vs builder API

- **Decision**: **derive** (`#[derive(Parser)]` + `#[derive(Subcommand)]`).
- **Rationale**: workspace 가 이미 `features = ["derive"]` 선언. rules/rust/cli.md 가 derive 를 표준으로 명시("Don't hand-roll arg parsing"). 명령 grammar 를 구조체 필드로 선언 → FR-003(단일 grammar)·SC-008(single source of truth) 직접 달성. ~50 명령에 builder 보일러플레이트는 오히려 drift 재유발.
- **Alternatives**: builder API — 런타임 동적 구성엔 유리하나 정적 ~50 명령엔 장황. lexopt/pico-args — 의존성 교체라 FR-011(신규 dep 0) 위반. **기각**.

## D2. 마이그레이션 sequencing — incremental wave vs big-bang

- **Decision**: **위험도순 incremental wave** (P1 hook → P2 데이터 → P3 분석). transition 중 `Commands` enum 이 미이관 명령을 raw `Vec<String>` passthrough variant 로 보유, 각 variant 는 기존 `cmd_*(&rest)` 호출. 바이너리는 wave 사이 항상 빌드·테스트 green.
- **Rationale**: 단일 3,300줄 파일 + ~50 명령을 한 번에 재작성하면 회귀 표면이 거대하고 review 불가. hook 경로(production 매 세션 실행)를 fail-open 안전망과 함께 먼저 옮기면 최대 위험을 조기 격리. passthrough variant 가 "coexistence" 를 보장해 매 PR 이 독립 mergeable.
- **clap 구현 메모**: passthrough 는 `#[command(external_subcommand)]` 가 아니라, 미이관 명령마다 명시적 variant + `#[arg(trailing_var_arg = true, allow_hyphen_values = true)] rest: Vec<String>` 로 raw 인자를 모아 기존 handler 에 전달. 이관 완료 시 해당 variant 를 typed args 구조체로 교체.
- **Alternatives**: big-bang(한 PR 에 전부) — review·rollback 악몽, 기각. 별도 바이너리 병행 후 swap — 과잉, FR-013(명령 set 동결)과 무관한 복잡도, 기각.

## D3. fail-open — clap 파싱 실패가 hook 경로로 새지 않게

- **Decision**: argv 를 clap 에 넘기기 전에 **subcommand 토큰을 peek 해 hook-class 분류** → `Cli::try_parse_from(argv)` → 에러 시 hook-class 따라 분기.
  - hook 명령(아래 set): 파싱 실패해도 **exit 0**, hook 출력 계약(allow JSON / `{}`) 유지.
  - 비-hook: clap 에러 메시지 stderr 출력 후 **exit 64**.
- **권위 fail-open set** (hooks.json + SessionStart wrapper + Clarifications 2026-05-29): **core 8개** `session-start`(SessionStart exec), `prompt-route`(UserPromptSubmit), `preauth-check`·`commit-gate`(PreToolUse Bash), `tdd-inject`(PreToolUse Edit|Write), `classify-exit`·`test-classifier`(PostToolUse[+Failure] Bash), `state-update --edit-event`(PostToolUse Edit|Write); **추가** `autowire-statusline`(SessionStart wrapper — W-detached 이나 classify()=fail-open), `karpathy-inject`(prompt-route 임베드 — fail-open 분류, typed 이관 P3). **예외**: `token-gate`(SKILL deploy gate)는 Normal — exit 0/65(unauthorized) 의미 보존, parse error→exit 64.
- **Parity scope 주의 (FR-001)**: fail-open 은 **hook event 경로**에 scope 돼요 — 토큰의 모든 CLI 호출에 무차별 exit 0 이 아니에요. flag 를 받는 hook 명령(특히 `state-update`)은 **유효한 hook flag 경로**(`--edit-event` 등)만 파싱 실패 시 fail-open 이고, malformed/비-hook 입력(`--bogus`, 무인자)은 **기존 exit 64 를 보존**해야 해요(현 `cmd_state_update` 가 unknown/missing flag 에 64 반환). 즉 classify() 결과를 "이 명령은 무조건 0" 으로 구현하면 안 되고, hook event 맥락에서만 0. (`classify-exit` 는 handler 가 unknown flag 를 무시(`_ => {}`)해서 애초에 64 를 안 내므로 무관.)
- **Rationale**: clap derive 의 기본 동작은 파싱 실패 시 `exit(2)` 자동 종료(handler 도달 전). hook 이 exit 2 로 끝나면 Claude Code 가 메인 흐름을 차단 → CLAUDE.md fail-open 계약 위반(가장 치명적 회귀). 에러 시점에 clap 이 subcommand 를 못 알려줄 수 있어 argv[1] 을 미리 peek 해 분류축을 확보. (분류축은 **subcommand 토큰**이지 hook-registry 이름이 아님 — `token-gate`↔`token-freshness-gate` 같은 불일치는 registry 가 아니라 토큰 기준이라 무관.)
- **구현**: 각 hook handler 는 이미 `hook_safety::is_hook_disabled(...)` + `unwrap_or` fail-soft 패턴 보유. clap 도입은 그 진입 전 파싱 레이어만 추가하므로, 파싱 레이어도 동일하게 hook-class 면 절대 비정상 종료 안 하게 감싸요.
- **Alternatives**: 모든 hook 명령에 극도로 관대한 schema(`allow_hyphen_values`, ignore unknown) 부여 — 부분적이나 unknown-subcommand·구조 오류는 여전히 누출, 불완전. 기각. 전역 `catch_unwind` — 파싱 실패는 panic 아니라 `Err`라 부적합. 기각.

## D4. clap `try_parse` ErrorKind 분기 (help/version vs usage error)

- **Decision** (Context7 `/clap-rs/clap` 확인):
  - `ErrorKind::DisplayHelp` (명시적 `--help`) → `e.print()` **stdout** + **exit 0**. (clap `e.exit()` 가 정확히 이 동작 — help 는 위임 가능.)
  - `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand` (인자/subcommand 없음) → **stderr** + **exit 64**. 기존 "subcommand 없으면 USAGE→stderr+64" 보존. clap 기본(stdout+0)과 다르므로 **명시 remap 필수**.
  - `ErrorKind::DisplayVersion` → D5 에서 pre-intercept 되어 여기 도달 안 함(도달 시 안전하게 exit 0).
  - 그 외(InvalidSubcommand·UnknownArgument·MissingRequiredArgument·InvalidValue 등) → hook-class 면 exit 0, 아니면 `e.print()` stderr + **exit 64**.
- **Rationale**: `e.exit()` 는 clap 기본 종료코드(usage=2)를 써서 SC-006(exit 64)·다수 test `status.code()==Some(64)` 와 충돌. 그래서 usage 계열은 `e.print()` 후 우리가 `process::exit(64)`. help 만 clap 위임.
- **Alternatives**: 전부 `e.exit()` 위임 — exit 2 누출로 test 다수 깨짐, 기각.

## D5. version 의도 pre-intercept (clap 이전)

- **Decision**: `version` / `--version` / `-v` 를 **clap 파싱 전에** 직접 가로채요. argv 전체를 스캔해 (a) version 토큰 존재 + (b) `--quiet` 존재(위치 무관) 판정 → quiet 면 빈 stdout·빈 stderr·exit 0, 아니면 `axhub-helpers {ver} (plugin v{ver}, schema v0)` stdout + exit 0. clap 의 auto `#[command(version)]` 는 **비활성**(`disable_version_flag`).
- **Rationale**: 계약(`version` subcommand | `--version` flag | `-v` | `--quiet` 어느 순서든, quiet⇒stdout·stderr 모두 빈)이 clap 의 flag/subcommand 모델과 적대적. `version_quiet_test.rs` 가 `version --quiet`·`--version --quiet` 양쪽 + 인자 순서 무관 + 빈 stderr 까지 lock. clap globals 로 표현 불가 → pre-intercept 가 유일하게 byte-parity 보장(FR-004). SessionStart wrapper 의 `timeout 3 ... --version --quiet || true` warmup 도 이 빈-출력에 의존.
- **Alternatives**: clap `ArgAction::Version` 커스텀 — `--quiet` any-order + 빈 stderr 재현 불가, 기각.

## D6. per-command 한국어 help/error 보존 (FR-006a)

- **Decision**: 두 채널로 보존.
  - **help 텍스트**: 큐레이션된 한국어 블록(예: `routing-stats` PRIVACY 안내)은 clap `#[command(long_about = "...")]` / `#[arg(long_help = "...")]` 에 한국어 그대로 넣어요. top-level `--help` 골격·`Usage:` 줄만 clap 영어.
  - **에러 메시지**: 한국어 도메인 에러(예: `consent-mint` 빈 stdin → exit 65 + 한국어 안내, `post-install` 한국어 flag 에러)는 **handler 레벨 검증**으로 유지 — clap 의 영어 auto-error 가 아니라 기존 `eprintln!` + 해당 exit code 경로 보존.
- **Rationale**: 한국어 사용자 UX + 가치 있는 안내(PRIVACY/stdin 가이드) 보존(clarify Q1 결정). clap 영어 terse auto-help 로 퇴화 금지. 도메인 검증 에러는 어차피 clap 의 타입 검증 범위 밖(stdin 내용·token 형식)이라 handler 에 남는 게 자연스러움.
- **Alternatives**: 전면 clap 영어 — clarify 에서 사용자가 기각. routing-stats 등만 선별 — 경계 모호, 기각.

## D7. `&[String]` lib 모듈 처리 (`_with_runner` seam 보존)

- **Decision**: lib 모듈(`resolve.rs`, `sync.rs`, `snippet.rs`, `bootstrap.rs`, `deploy_prep.rs`)의 **공개 시그니처(`run_*(&[String])` + `run_*_with_runner`)는 보존**. clap args 구조체는 main.rs/cli 레이어에서 파싱하고, 해당 명령 handler 가 구조체 → 기존 lib 호출로 연결. lib 내부 파싱 제거는 wave 내 opportunistic cleanup(필수 아님).
- **Rationale**: `_with_runner` 변형은 테스트 의존성 주입 seam(예: `run_resolve_with_runner`, `run_deploy_prep_with_runner`). 시그니처를 깨면 다수 단위 테스트가 깨져 parity oracle 훼손. SC-004 는 "subcommand **dispatch 진입점**"의 `while i<args.len()` 제거를 측정 — lib 내부 파싱은 2차 목표. `list_deployments`(이미 `ListDeploymentsArgs` 타입)·`preflight`(무인자)는 이관 단순.
- **Alternatives**: lib 전부 clap 구조체로 즉시 전환 — seam 파괴 + scope 폭증, 기각.

## D8. binary size / startup 영향

- **Decision**: clap 링크로 인한 binary size 증가를 **측정·보고**(blocker 아님). Wave 4 에서 5개 cross-arch 타겟의 `ls -la` + `cargo bloat --release` before/after 기록. startup 은 parse µs vs spawn ms 라 무시 가능 — 측정만, 목표 회귀 0.
- **Rationale**: clap 은 현재 선언만 됐을 뿐 미링크(소스 미사용). 채택 시 실제 링크되어 +수백 KB 예상. release.yml 이 5개 서명 바이너리를 배포하므로 다운로드 크기 영향을 가시화해야 함(spec Deferred 항목). hook startup 은 이미 프로세스 spawn(수 ms)이 지배적이라 파싱 추가비용 무의미.
- **Alternatives**: `clap` feature 최소화(`std` 만, `color`/`suggestions`/`usage` 제거) — size 줄이나 영어 usage-error UX 저하 가능. Wave 4 측정 후 필요 시 feature 조정으로 미뤄요.

## D9. edition / MSRV

- **Decision**: edition **2021** + MSRV **1.83** 유지. clap 4 derive 는 이 위에서 동작. let-chains(1.88)·`if let` match guard(1.95)·edition 2024 기능 **금지**.
- **Rationale**: workspace 가 `edition="2021"`, `rust-version="1.83"`, `resolver="2"` 고정. MSRV 인상은 정책 결정(rules/rust/coding-style.md)이라 이 리팩토링 scope 밖. clap 4 MSRV 는 1.83 이하라 안전.
- **Alternatives**: edition 2024 승격 — scope 무관 + CI/릴리스 영향, 기각.

---

## 미해결 / Wave-defer 항목

- **버그성 불일치 목록(FR-013 예외)**: 마이그레이션 중 USAGE↔dispatch 불일치(예: USAGE 미기재 hidden 명령) 발견 시 별도 목록화. 현재까지 알려진 것: hidden 명령(`state-show`, `state-update`, `commit-gate`, `test-classifier`, `tdd-inject`, `karpathy-inject`, `consent`)이 USAGE 에 없음 → **노출 안 함**(hidden 유지)이 parity, 정정 아님. 실제 버그 후보 0 (모두 의도된 hidden).
- **clap feature 조정**: D8 측정 결과 따라 Wave 4 에서 판단.

# Phase 1 Data Model: axhub-helpers clap 리팩토링

**Date**: 2026-05-29 | **Plan**: [plan.md](./plan.md)

> 이 리팩토링은 데이터 엔티티가 아니라 **CLI 파싱 구조**를 다뤄요. 아래는 도입할 내부 타입. 명령별 grammar(이름·flag·positional)는 중복 회피 위해 [contracts/cli-commands.md](./contracts/cli-commands.md) 에만 둬요.

## 1. `Cli` — top-level parser

```rust
#[derive(clap::Parser)]
#[command(
    name = "axhub-helpers",
    bin_name = "axhub-helpers",
    disable_version_flag = true,   // D5: version 은 pre-intercept
    // about/long_about: top-level 영어 골격 (FR-006)
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
```

- `disable_version_flag = true` — clap auto `--version`/`-V` 끄고 D5 pre-intercept 로 대체.
- help flag(`--help`/`-h`)는 clap 기본 유지 → `DisplayHelp` 에러로 stdout+exit 0 (D4).

## 2. `Commands` — subcommand enum (transition 형태)

```rust
#[derive(clap::Subcommand)]
enum Commands {
    // ── 이관 완료: typed args ──
    SessionStart,                          // 무인자 hook
    Preflight,
    Redact,
    Statusline,
    Path(PathArgs),                        // positional kind
    DeployPrep(DeployPrepArgs),
    RoutingStats(RoutingStatsArgs),
    // ... wave 진행에 따라 추가

    // ── 미이관: raw passthrough (D2) ──
    // transition 동안만. 기존 cmd_*(&rest) 로 위임.
    #[command(external_subcommand)]
    Passthrough(Vec<String>),              // 또는 명령별 명시 variant + trailing_var_arg
}
```

- **transition 불변식**: 모든 wave 사이 enum 은 (typed ∪ passthrough) 로 ~50 명령 전체를 커버 → 빌드·테스트 항상 green.
- **hidden 명령**(`state-show`, `consent`, `karpathy-inject` 등)은 `#[command(hide = true)]` variant 로 — 동작 유지하되 `--help` 미노출 (FR-007).
- **별칭**: `#[command(visible_alias = "...")]` / `alias`. (version 별칭은 enum 밖 pre-intercept.)
- **중첩**: `Config(ConfigArgs)` 의 `ConfigArgs` 가 `#[command(subcommand)] sub: ConfigSub { Get{..}, Set{..} }`; `bootstrap dependency-plan`, `diagnose hitl` 동일 패턴.

## 3. `ExitCode` — 종료코드 분류

바이너리 **자체** process exit (sysexits.h 관례):

| 값 | 의미 | 비고 |
|----|------|------|
| `0` | 성공 | hook 명령은 실패해도 항상 이 값(fail-open) |
| `64` | EX_USAGE — 알 수 없는 subcommand/flag, 필수 인자 누락, subcommand 미지정 | clap 기본 2 를 여기로 remap (D4, SC-006) |
| `65` | EX_DATAERR — 입력/데이터 오류 (예: consent-verify 실패, token 없음, 잘못된 stdin) | handler 반환 |

> **주의 — 자체 exit 아님**: `124`(timeout)·`127`(spawn 실패)·`70` 은 axhub-helpers 가 **shelled-out `axhub` CLI 결과**(`CliOutput`)나 **data-layer 결과**를 JSON 으로 *보고*하는 값이지, axhub-helpers 자신의 process 종료코드가 아니에요. 파싱 레이어 변경과 무관 — 보존 대상이지만 분류축이 다름.

구현: 별도 enum 강제 안 함. 기존 handler 가 `anyhow::Result<i32>` 반환 → `main` 이 `process::exit(code)`. clap parse-error 경로만 새로 64/0 매핑.

## 4. `HookClass` — fail-open 분류 (D3)

argv[1](subcommand 토큰) 기준 분류. **registry 이름 아님**(토큰 축).

```rust
enum HookClass {
    /// CC hook 또는 SessionStart exec — 파싱 실패해도 exit 0 (출력 계약 유지)
    FailOpenHook,
    /// 일반 명령 — 파싱 실패 시 stderr + exit 64
    Normal,
}

fn classify(subcommand_token: Option<&str>) -> HookClass { /* set 매칭 */ }
```

**FailOpenHook set** (hooks.json + SessionStart wrapper 권위, Clarifications 2026-05-29 확정):
`session-start`, `prompt-route`, `preauth-check`, `commit-gate`, `tdd-inject`, `classify-exit`, `test-classifier`, `state-update`, `autowire-statusline`, `karpathy-inject`.

**분류 주석**:
- `autowire-statusline` — contracts 에선 class=W(wrapper-detached, exit 흡수)지만 classify() 는 **fail-open 으로 분류**해요 — parse error 도 exit 0 (방어적; SessionStart 맥락에서 안전). W(호출 기제)와 fail-open(exit 동작)은 직교 축이라 양립해요.
- `karpathy-inject` — `prompt-route` 임베드 + hidden 이지만 hook 맥락이라 **fail-open 분류**(parse error→exit 0). typed 이관은 P3.
- `token-gate`(registry `token-freshness-gate`) — **유일한 boundary 예외 = Normal 분류**. SKILL deploy gate 라 exit 0/65(unauthorized=65) 의미 보존 → fail-open(항상 0) 아님, parse error→exit 64.

> **Parity scope (FR-001)**: `FailOpenHook` 은 **hook event 경로**에만 exit 0 을 적용해요. flag 를 받는 hook 명령(특히 `state-update`)은 유효 hook flag(`--edit-event`)만 fail-open 이고 malformed/비-hook 입력(`--bogus`/무인자)은 **기존 exit 64 보존**(현 `cmd_state_update` 동작). classify()→FailOpenHook 를 "무조건 exit 0" 으로 구현하면 parity 깨짐. (`classify-exit` 는 handler 가 unknown flag 무시라 무관.)

## 5. 모듈 배치 (data flow)

```
argv
 └─> cli::run()
      ├─ enable_utf8_console()        # main.rs, 파싱 전 (FR-012)
      ├─ version pre-intercept        # D5 — clap 이전
      ├─ classify(argv[1]) -> HookClass
      ├─ Cli::try_parse_from(argv)
      │    ├─ Ok(cli)  -> dispatch(cli.command)  # typed -> handler / lib 호출
      │    └─ Err(e)   -> handle_parse_error(e, hook_class)  # D4 분기
      └─ process::exit(code)
```

- `dispatch` 는 기존 `cmd_*` handler 와 lib `run_*` 호출로 연결. lib 시그니처·`_with_runner` seam 보존(D7).

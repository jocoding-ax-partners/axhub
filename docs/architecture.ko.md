# axhub 아키텍쳐 & 개발자 온보딩

> 이 문서는 **이 플러그인을 개발·확장하려는 개발자를 위한 심화 온보딩 문서**예요.
> 설치·셋업·사용법만 필요하면 [README](../README.md) 를 봐요.

**상태**: v0.9.37 · preview-first destructive workflow · 5 cross-arch cosign-signed binary 라이브.

---

## 이 문서 읽는 법

처음 온보딩하는 개발자라면 이 순서를 권해요:

1. **[§1 제품 개요](#1-제품-개요--무엇을-왜)** — 이게 무슨 도구이고 왜 이렇게 생겼는지 (5분)
2. **[§2 아키텍쳐](#2-아키텍쳐)** — 전체 레이어 멘탈 모델 + repo 지도 (15분)
3. **[§5 e2e 플로우](#5-e2e-플로우)** — "배포해" 한마디가 끝까지 흐르는 과정 (10분)
4. **[§4 동작 원리](#4-동작-원리)** — 라우팅·SKILL·명시 승인·auth 같은 메커니즘 깊이 파기
5. **[§6 개발 워크플로우](#6-개발-워크플로우)** — 직접 SKILL/hook/서브커맨드 추가하고 디버깅하기

급하면 **[§10 레퍼런스](#10-레퍼런스)** 의 전체 SKILL 표 / env var 표 / 용어집부터 봐도 돼요.

> 본문은 코드에서 실제로 확인한 사실만 담았어요. 핵심 주장에는 `파일:라인` 출처를 붙였으니 직접 열어보면서 검증해요.

---

## 1. 제품 개요 — 무엇을, 왜

### 무엇을 하나

axhub SaaS 를 도입한 회사의 **바이브코더 직원**이 Claude Code 안에서

> "결제 앱 만들어줘" → "GitHub 연결해" → "배포해" → "결과 봐"

같은 **한국어 자연어**로 앱 lifecycle 전체를 수행해요. 슬래시 명령(`/axhub:deploy`, 한글 alias `/axhub:배포`)도 같은 워크플로우를 불러요.

여기에 v1.0 라인부터는 **코드 품질 보조**(리뷰/디버그/TDD/배포 게이트)까지 더해져서, 단순 배포 도구를 넘어 vibe coder 의 작업 흐름 전반을 받쳐줘요.

### 왜 이렇게 생겼나 — 핵심 철학

axhub 플러그인의 모든 설계는 한 문장으로 요약돼요:

> **플러그인은 얇은 라우팅 레이어다. 비즈니스 로직은 전부 `ax-hub-cli`(외부 CLI)와 backend 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령 매핑, (2) 안전한 기본값 강제, (3) exit code 기반 자동 복구 안내만 담당한다.**

이 원칙이 코드베이스 곳곳을 지배해요:

- 플러그인은 backend(`axhub-api`)나 MCP 를 **직접 호출하지 않아요**. 항상 `ax-hub-cli` 를 거쳐요.
- 플러그인의 Rust helper(`axhub-helpers`)는 인증·배포 로직을 재구현하지 않고 CLI 를 **invoke** 하거나 그 결과를 **분류·복구 안내**할 뿐이에요.
- 그래서 CLI 가 새 기능을 내면 플러그인은 NL 트리거 어구와 안전 가드만 추가하면 돼요.

새 코드를 짤 때 "이건 CLI 가 해야 하나, 플러그인이 해야 하나?"를 항상 먼저 물어요. 답이 "비즈니스 로직"이면 거의 CLI 쪽이에요.

---

## 2. 아키텍쳐

### 2.1 5-레이어 큰 그림

```
┌──────────────────────────────────────────────────────────────────────┐
│ ① 사용자 (vibe coder) — 한국어 자연어                                 │
│    "내 paydrop 앱 배포해" · /axhub:배포                                │
└───────────────────────────────┬──────────────────────────────────────┘
                                │ UserPromptSubmit
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ② Claude Code (모델은 SKILL frontmatter 의 model: 로 라우팅)          │
│                                                                      │
│   hooks/        SessionStart·UserPromptSubmit·PreToolUse·PostToolUse  │
│                 → 전부 bin/axhub-helpers 서브커맨드로 위임 (fail-open) │
│   skills/       32개 SKILL.md — 자연어 트리거 워크플로우              │
│   commands/     9개 슬래시 (+ 한글 alias)                            │
│   agents/       8개 agent md (3 quality + 5 migrate planning)       │

└───────────────────────────────┬──────────────────────────────────────┘
                                │ Bash tool 호출
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ③ bin/axhub-helpers (Rust 단일 바이너리, crates/axhub-helpers)        │
│    preflight · deploy-prep · resolve · preview-first approval   │
│    prompt-route · classify-exit · session-start · token-gate ...      │
│    → 모든 hook 진입점 + SKILL 의 in-body preflight 가 이 바이너리를 호출 │
└───────────────────────────────┬──────────────────────────────────────┘
                                │ 얇은 wrapper — 비즈니스 로직 위임
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ④ ax-hub-cli (외부 Rust CLI, 검증 surface v0.17.4 / 지원 범위 v0.17.3 ~ <1.0.0) │
│    auth login/status · apps · deploy create/status/logs · catalog ... │
│    계약: 모든 호출에 --json, 구조화된 exit code(0/1/64/65/66/67/68)    │
└───────────────────────────────┬──────────────────────────────────────┘
                                │ HTTPS (TLS-pinned)
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ⑤ axhub-api backend — https://axhub-api.jocodingax.ai                 │
│    /api/v1/... (tenants · apps · deployments · catalog/resources)     │
└──────────────────────────────────────────────────────────────────────┘

별도 레이어 — 사용자 프로젝트 안의 AI 컨텍스트:
  .axhub/  AXHUB.md(AI 규칙) · AXHUB_TARGET(모드) · catalog.json(권한 스냅샷, gitignore)
```

레이어 사이 경계가 곧 이 코드베이스의 책임 분리선이에요. "어디를 고쳐야 하지?"는 거의 항상 "어느 레이어 문제지?"로 환원돼요.

### 2.2 "얇은 라우팅 레이어" 가 코드에서 의미하는 것

| 플러그인이 하는 일 | 플러그인이 안 하는 일 |
|--------------------|----------------------|
| 자연어 → SKILL 매칭 (Claude Code native) | 인증 토큰 발급 (→ ax-hub-cli) |
| 안전 기본값 강제 (preview card, D1 guard) | 배포 오케스트레이션 (→ ax-hub-cli) |
| destructive 명령 preview-first 확인 | backend 직접 호출 (→ ax-hub-cli) |
| exit code → 한국어 복구 안내 | MCP 서버 (v1.1 범위 밖, deferred) |
| preflight 상태 주입 | 데이터 카탈로그 쿼리 실행 (→ ax-hub-cli) |

### 2.3 repo 디렉토리 지도

```
axhub/
├── .claude-plugin/
│   ├── plugin.json          # 플러그인 메타데이터 (name/version/author/license)
│   └── marketplace.json     # 마켓플레이스 등록 엔트리 — Claude Code 가 이걸로 플러그인 발견
├── skills/<name>/SKILL.md    # 32개 NL 트리거 워크플로우 (+ _template 스캐폴드 원본)
├── commands/*.md             # 9개 슬래시 명령 (deploy/배포/login/status/logs/apps/doctor/update/help)
├── agents/*.md               # 8개 agent md (quality 3 + migrate planning 5)

├── hooks/
│   ├── hooks.json            # 모든 hook 이벤트 → 명령 매핑 (단일 등록 파일)
│   ├── session-start.sh/.ps1 # SessionStart 부트스트랩 (binary 자동설치·warmup·token-init)
│   ├── session-start-autowire.sh/.ps1  # statusLine 자동 설정 (백그라운드)
│   └── axhub-helpers.sh      # portable helper resolver (binary 부재 fail-open)
├── crates/axhub-helpers/     # Rust helper 바이너리 (이 플러그인의 코어)
│   ├── src/main.rs           # argv 디스패치 + 모든 서브커맨드 핸들러 (~3000줄)
│   ├── src/*.rs              # 모듈 (preflight/hook_safety/runtime_paths/keychain/...)
│   ├── data/catalog.json     # exit-code → 한국어 공감 메시지 카탈로그 (컴파일 임베드)
│   └── tests/*.rs            # cargo 통합 테스트
├── scripts/*.ts              # 빌드·codegen·lint·release 툴링 (Bun)
├── tests/                    # bun test + routing corpus + e2e 매트릭스
├── docs/                     # 설계 문서 (plugin-developer-guide / HOOKS / adr/ ...)
├── bin/                      # 빌드 산출물 (axhub-helpers + 플랫폼별 바이너리 + install.sh/ps1)
├── CLAUDE.md / AGENTS.md     # AI 에이전트용 작업 규칙 (skill authoring / release 계약)
└── package.json / Cargo.toml # Bun 툴링 / Rust workspace
```

> **기억할 점**: TypeScript 는 *repo 툴링*(빌드·codegen·테스트) 전용이에요. 런타임 helper 는 v0.2.0 부터 Rust 단일 바이너리 하나뿐이에요(TS shadow 제거). `package.json` 의 `build` 도 실제로는 Cargo 빌드를 감싸는 Bun 스크립트예요.

### 2.4 Rust helper(`axhub-helpers`) — 코어 해부

`crates/axhub-helpers/src/main.rs` 가 진입점이에요. `clap` 의존성이 있지만 디스패치는 **손으로 짠 `match`** 예요: `args[0]` 을 서브커맨드로 보고 핸들러로 분기해요 (`main.rs:74` `fn run()`). 각 핸들러는 `i32` exit code 를 돌려주고 그게 프로세스 exit code 가 돼요.

**자주 만지는 서브커맨드** (전체 ~40개 중):

| 분류 | 서브커맨드 | 역할 | 위치 |
|------|-----------|------|------|
| **hook 진입점** | `session-start` | 환영 메시지 + session-bundle 작성 + quality 컨텍스트 주입 | `main.rs:1644` |
| | `prompt-route` | UserPromptSubmit: preflight + audit 만 (라우팅 안 함, §4.1) | `main.rs:1085` |
| | `classify-exit` | PostToolUse: axhub CLI exit code → 한국어 공감 메시지 | `main.rs:703` |
| | `commit-gate` | PreToolUse: 리뷰 안 한 commit/push 에 ask | `main.rs:872` |
| | `tdd-inject` / `test-classifier` | TDD 리마인더 / 테스트 실패 분류 | `main.rs:933` / `902` |
| **배포 파이프라인** | `preflight` | CLI·인증·앱·환경 probe → JSON (§4.3) | `preflight.rs:398` |
| | `deploy-prep` | preflight+resolve+bootstrap-plan 병렬 1콜 | `main.rs:2386` |
| | `resolve` | git+CLI 로 앱 slug/배포 컨텍스트 해석 | `resolve.rs` |
| | `token-gate` | 배포 직전 토큰 신선도 게이트 (exit 65 = 만료) | `main.rs:249` |
| | `list-deployments` | **helper 의 유일한 직접 HTTP 호출** — axhub-api 배포 목록 (preflight/resolve 는 직접 HTTP 가 아니라 `axhub` CLI 를 shell-out 해서 상태를 읽어요) | `main.rs:1768` |
| **AI 컨텍스트** | `sync` | `.axhub/` 컨텍스트 디렉토리 작성 | `sync.rs:27` |
| | `snippet` | 유저 앱용 connector 코드 스니펫 생성 (Mode A/B) | `snippet.rs:26` |
| **UX/설정** | `settings-merge` / `autowire-statusline` | `settings.json` statusLine 병합 | `main.rs:2753` / `2517` |
| | `statusline` / `doctor` / `routing-stats` | statusline 렌더 / 진단 JSON / 라우팅 통계 | — |

**주요 모듈** (`src/lib.rs` 에 선언):

| 모듈 | 책임 |
|------|------|
| `preflight` | CLI/인증/앱/환경 probe → `PreflightOutput` (병렬 thread::scope) |
| `hook_safety` | fail-open kill switch (`is_hook_disabled`) + 에러 로깅 (`append_hook_error`) |
| `runtime_paths` | XDG state/runtime 경로와 private 파일 I/O 공통 유틸 |
| `keychain` / `keychain_windows` | macOS `security` / Linux `secret-tool` / Windows PowerShell 토큰 읽기 |
| `catalog` | exit-code → 공감 메시지 (로컬, `data/catalog.json` 컴파일 임베드) |
| `list_deployments` | 캐노니컬 `axhub deploy list --json` 을 감싸는 CLI wrapper (subprocess) |
| `audit` | 라우팅 audit 로그 (sha256 해시만, 7일 회전) |
| `telemetry` / `event_log` / `observability` | phase marker / 배포 이벤트 NDJSON |
| `quality_state` / `commit_gate` / `tdd_inject` / `karpathy_inject` | v1 품질 자동모드 |
| `statusline` / `autowire` / `orphan_stub` | statusLine 렌더·자동설정·고아 stub |
| `runtime_paths` | XDG 경로 해석 (token/state/cache) |

> 서브커맨드 전체 카탈로그·hook 호출 계약(stdin/stdout JSON)·전체 모듈 맵·외부 의존성은 **[부록 A — axhub-helpers 심화 레퍼런스](#부록-a--axhub-helpers-심화-레퍼런스)** 에 정리했어요.

### 2.5 상태·데이터가 사는 곳 (XDG 경로)

디버깅할 때 이 경로들을 알면 "hook 은 떴는데 아무 일도 안 일어났어요" 같은 미스터리를 빨리 풀어요.

| 무엇 | 경로 |
|------|------|
| 플러그인 인증 토큰 | `$XDG_CONFIG_HOME/axhub-plugin/token` |
| helper state root | `${XDG_STATE_HOME:-$HOME/.local/state}/axhub` (runtime_paths 공통 해석) |
| hook 에러 로그 | `$XDG_STATE_HOME/axhub-plugin/hook-errors.jsonl` (0o600, 7일 회전) |
| session bundle | `$XDG_STATE_HOME/axhub-plugin/session-bundle.json` (TTL 300초) |
| 라우팅 audit | 로컬 audit 로그 (sha256 해시만, 7일, 외부전송 X) |
| 마지막 배포 캐시 | `~/.cache/axhub-plugin/last-deploy.json` |
| 품질 상태 | `.axhub-state/quality.json` (프로젝트 로컬, commit 금지) |

---

## 3. 현재 하네스

"하네스"는 이 플러그인을 **돌리고·만들고·검증하고·출시하는** 네 겹의 자동화예요.

### 3.1 런타임 하네스 — hook → helper

Claude Code 가 플러그인을 실행하는 통로는 `hooks/hooks.json` 단 하나예요. 5개 이벤트가 등록돼 있고, 거의 전부 `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers <서브커맨드>` 로 위임해요.

| 이벤트 | matcher | 실행 | timeout |
|--------|---------|------|---------|
| `SessionStart` | — | `hooks/session-start.sh` | 30s |
| `SessionStart` | — | `hooks/session-start-autowire.sh` | 10s |
| `UserPromptSubmit` | — | `axhub-helpers prompt-route` | 5s |
| `PreToolUse` | `Bash` | `axhub-helpers commit-gate` | 5s |
| `PreToolUse` | `Edit\|Write\|MultiEdit\|NotebookEdit` | `axhub-helpers tdd-inject` | 5s |
| `PostToolUse` | `Bash` | `axhub-helpers classify-exit` | 5s |
| `PostToolUse` | `Bash` | `bash hooks/axhub-helpers.sh verify-deploy-artifact` | 7s |
| `PostToolUse` | `Bash` | `axhub-helpers test-classifier` | 5s |
| `PostToolUse` | `Edit\|Write\|MultiEdit\|NotebookEdit` | `axhub-helpers state-update --edit-event` | 5s |
| `PostToolUseFailure` | `Bash` | `axhub-helpers test-classifier` | 5s |

> `Stop` hook 은 없어요. `hooks/post-commit` 는 Claude Code hook 이 아니라 git post-commit hook(설치는 `post-install` 서브커맨드)이에요.

**fail-open 계약** (`docs/HOOKS.md` 가 정본):

1. **어떤 실패에서도 exit 0** — binary 없음, 네트워크 실패, config 손상, 권한 거부, panic 어느 것도 메인 흐름을 막지 않아요.
2. **에러는 `systemMessage` 로만 노출** — stdout 에 `{"systemMessage":"..."}` 를 찍으면 Claude Code 가 채팅에 보여줘요.
3. **panic 금지** — `Result<>` + `unwrap_or_else`/`?` 패턴. 외부 입력 처리부만 `catch_unwind`.
4. **non-interactive 컨텍스트는 조용히 skip** — TTY 없음/CI → `systemMessage` 없이 그냥 exit 0.
5. **디버그 흔적 보존** — 실패는 `hook-errors.jsonl` 에 atomic append (`{ts,hook,error}`).

모든 hook 핸들러의 **첫 줄**이 `hook_safety::is_hook_disabled("name")` 이에요 (`hook_safety.rs:52`). 이게 kill switch 의 단일 관문이에요.

**kill switch 우선순위** (`hook_safety.rs:52-66`):

```
AXHUB_DISABLE_HOOKS=1        # 모든 hook off (최우선)
  > AXHUB_DISABLE_HOOK=a,b   # 지정 hook 만 off (csv)
    > DISABLE_AXHUB=1        # legacy alias (deprecated, stderr 경고 1회)
```

truthy 값: `1`/`true`/`yes`/`on`. shell wrapper(`session-start.sh`, `axhub-helpers.sh` binary-missing fail-open) 와 Rust(`hook_safety.rs`) 가 동일한 kill-switch 계약을 지켜요 — shell 에서 먼저 거르면 binary cold-start 비용도 안 들어요.

기능별 opt-out 도 있어요: `AXHUB_DISABLE_STATUSLINE_AUTOWIRE` / `AXHUB_DISABLE_TRIGGERS`(품질) / `AXHUB_DISABLE_MEGASKILL` / `AXHUB_DISABLE_KARPATHY` / `AXHUB_DISABLE_POSTCOMMIT` / `AXHUB_NO_AUDIT`.

### 3.2 빌드 하네스

런타임 helper 는 Rust 로 짜고 Cargo 로 빌드해서 `bin/` 에 떨궈요. `bun run build` 는 `scripts/build-rust-helper.ts` 를 실행하는 얇은 래퍼일 뿐이에요.

**5개 cross-arch 타깃** (`build:all`):

| 산출 asset | Rust 타깃 |
|------------|-----------|
| `axhub-helpers-darwin-arm64` | `aarch64-apple-darwin` |
| `axhub-helpers-darwin-amd64` | `x86_64-apple-darwin` |
| `axhub-helpers-linux-amd64` | `x86_64-unknown-linux-gnu` |
| `axhub-helpers-linux-arm64` | `aarch64-unknown-linux-gnu` (CI 는 `cross` 사용) |
| `axhub-helpers-windows-amd64.exe` | `x86_64-pc-windows-msvc` |

호스트 빌드는 `bin/axhub-helpers`(런타임 이름)와 `bin/axhub-helpers-<host>`(release asset 이름) 두 벌을 써요.

**런타임 바이너리 선택(helper-pick)**: SKILL 과 hook 은 같은 2단 fallback 으로 바이너리를 찾아요 —

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
```

플러그인 bin 이 없으면 PATH 의 `axhub-helpers` 로 떨어져요.

**codegen — 단일 진실원천(SoT)에서 파생물 동기화**:

| 스크립트 | SoT | 무엇을 sync |
|----------|-----|-------------|
| `codegen:catalog` | `data/catalog.json` | deploy SKILL 의 error-empathy 카탈로그 생성 |
| `codegen:version` | `package.json` version | `install.sh`/`install.ps1`/`Cargo.toml` 의 버전 리터럴 |
| `codegen:skill-keywords` | Rust 키워드 테이블 | SKILL `description:` frontmatter 의 NL 트리거 어구 |
| `codegen:skill-examples` | `corpus.100.jsonl` | SKILL frontmatter `examples:` 블록 |

drift 는 `tests/codegen*.test.ts` 가 잡아요. 그래서 버전 같은 건 **손으로 고치면 안 되고** codegen 을 돌려야 해요.

### 3.3 테스트 하네스

| 종류 | 명령 | 무엇을 검증 |
|------|------|-------------|
| 단위/통합 (TS) | `bun test` | manifest·SKILL·workflow·codegen·hook·UX 회귀 (~80+ 파일) |
| 라우팅 corpus | `bun run test:routing` | NL→SKILL 매칭 정확도 vs claude-native 베이스라인 |
| e2e (staging) | `bun run test:e2e` | 실제 staging endpoint read-only probe |
| e2e (plugin) | `bun run test:plugin-e2e` | `claude -p` 매트릭스 — helper lifecycle |
| Rust | `bun run cargo:test` | helper 단위/통합/phase parity |
| 커버리지 | `bun run cargo:coverage` | `cargo llvm-cov` ≥85% line |

**라우팅 corpus eval** 이 이 프로젝트의 독특한 점이에요. `tests/corpus.100.jsonl`(115행)·`tests/corpus.20.jsonl`(24행)의 발화→기대 skill 쌍을 두 베이스라인(`docs-only` vs `claude-native`)으로 채점해서 **정확도 ≥95% 이고 drift ≤5%** 면 통과해요 (`tests/routing-score.ts`). 전체 `corpus.jsonl`(350행)은 advisory 라 실패해도 exit 0 이에요 (파일명의 20/100 은 nominal tier 라벨이라 실제 행수와 달라요). PR 은 `routing-drift.yml` 이 이 게이트를 강제해요.

**e2e 2종 구분**:
- `test:e2e` = Bun 테스트가 진짜 staging 을 친다 (토큰 secret 필요).
- `test:plugin-e2e` = `tests/e2e/claude-cli/run-matrix.sh` 가 tier 별 케이스를 돌린다. **T2**(PR 차단, API 비용 $0 — helper 바이너리만), **T1/nightly**(`claude -p` + 실제 API key, ~$1.5/run).

### 3.4 CI 하네스 (`.github/workflows/`)

| workflow | 트리거 | 게이트 |
|----------|--------|--------|
| `rust-ci.yml` | PR/push | 3-OS `cargo test`, clippy, llvm-cov ≥85%, Windows static CRT |
| `routing-drift.yml` | PR | corpus.100 ≥95% acc / ≤5% drift (실패 시 한국어 PR 코멘트) |
| `cross-platform-helper.yml` | PR (hook/bin 경로) | ubuntu+macos hook kill-switch / token-gate 테스트 |
| `perf-gate.yml` | PR (helper/deploy 경로) | prompt-route/preflight p95 천장 |
| `claude-cli-e2e.yml` | PR + nightly | T2(PR) / T1+T2+T3(nightly 02:00 KST) |
| `windows-smoke.yml` | tag/dispatch | install.ps1 dry-run, session-start.ps1 fail-open, CredReadW |
| `rust-staging-gates.yml` | PR + nightly | fmt/clippy/test/build/tsc + staging read-only e2e |
| `release.yml` | `v*` tag push | 5 binary 빌드 + cosign 서명 + GH release + Slack |

### 3.5 릴리스 하네스 — 2단계 + cosign

릴리스는 **`bun run release` 한 번이 아니라 2단계**예요. v0.9.1 회귀(tag 가 narrative amend 전 commit 을 가리켜서 release.yml 이 빈 narrative 로 발사) 때문에 분리됐어요. 그래서 `.versionrc.json` 에 `skip.tag=true` 가 있어요 — bump 단계는 commit 만 만들고 tag 는 안 만들어요.

```bash
# 1단계: 3파일 bump + postbump(codegen:version + release:check) + CHANGELOG + commit (tag X)
bun run release

# 2단계: CHANGELOG 에 한국어 narrative(해요체, ≥50자) 추가 후 bump commit 에 흡수
vim CHANGELOG.md
git commit --amend --no-edit -a

# 3단계: narrative 검증 → HEAD 에 tag 생성 → main+tag push
bun run release:tag
```

tag 가 push 되면 `release.yml` 이 5개 바이너리를 매트릭스 빌드하고, `manifest.json`+`checksums.txt` 를 만들고, Sigstore **keyless cosign 서명**(`.sig`+`.pem`)을 붙여 GitHub Release 에 올리고, narrative 를 Slack 에 전송해요.

> 자세한 계약은 [`AGENTS.md`](AGENTS.md) "Release Workflow" 와 [`docs/RELEASE.md`](docs/RELEASE.md) 에 있어요. `commit-and-tag-version` 을 쓰고 release-please/semantic-release 는 쓰지 않기로 ADR 결정됐어요.

---

## 4. 동작 원리

### 4.1 자연어 라우팅 — "라우터가 라우팅을 안 한다"

신규 개발자가 가장 많이 헷갈리는 부분이에요. `main.rs` 의 `prompt-route` 라는 이름만 보면 여기서 발화를 SKILL 로 매칭할 것 같지만, **안 해요.**

`cmd_prompt_route()` (`main.rs:1085`)의 실제 본문은 이게 전부예요:

```
is_hook_disabled 체크 → 발화 읽기 → run_preflight() → audit 기록(sha256 해시만)
→ <axhub-preflight-status> 컨텍스트 블록 생성 → (옵션) Karpathy 컨텍스트 → stdout
```

소스 주석이 직접 못 박아요:

```rust
// Approach E (Phase 2): cmd_prompt_route is preflight + audit only.
// No keyword chain, no skill enforcement ...
// Claude Code matches skills via SKILL.md frontmatter description natively
// (Phase 1 codegen merged main.rs phrases into descriptions).
```

**그럼 실제 라우팅은 누가 하나?** Claude Code 자체가 각 SKILL 의 `description:` frontmatter 를 보고 native 매칭해요. preflight hook 은 그 옆에 "CLI·인증 상태가 건강한지"만 컨텍스트로 주입해서, 모델이 SKILL 을 고른 뒤 헛발질 안 하게 도울 뿐이에요.

**왜 이런 구조인지 (온보딩 일화)**: 예전엔 `detect_prompt_route()` 라는 Rust 키워드 라우팅 함수가 있었어요. Phase 1 에서 codegen(`scripts/codegen-skill-keywords-from-rust.ts`)이 그 함수의 키워드 어구 ~300개를 각 SKILL `description` 으로 병합했고, Phase 2 에서 그 함수를 **통째로 삭제**했어요(지금 `main.rs` 에 `detect_prompt_route` 는 없어요). 그래서 트리거 어구의 진실원천은 이제 SKILL frontmatter 이고, `lint:keywords` 베이스라인이 그 어구가 톤 마이그레이션 등으로 바뀌는 걸 막아요. → SKILL `description` 의 트리거 어구를 함부로 바꾸면 라우팅이 깨져요.

### 4.2 SKILL 해부

각 `skills/<name>/SKILL.md` 는 YAML frontmatter + Markdown 워크플로우 본문이에요.

**frontmatter 계약**:

```yaml
---
name: deploy                    # 디렉토리명과 일치
description: '...배포해...ship...deploy...'  # NL 트리거 어구 (라우팅 진실원천, 잠금)
examples:                       # codegen/테스트 픽스처용 발화→인텐트 쌍
  - utterance: "paydrop 배포해"
    intent: "deploy current branch"
multi-step: true                # 4+ 단계 → 본문에 TodoWrite Step 0 필수
needs-preflight: true           # live state 필요 → 본문에 CANONICAL_PREFLIGHT_BLOCK 필수
allows-dependency-execution: false  # npm/bun install 류 허용 여부
model: sonnet                   # haiku|sonnet|opus
---
```

본문에 강제되는 패턴 4가지 (전부 `bun run skill:doctor` 가 검사):

1. **in-body preflight (ADR-0013)** — `needs-preflight: true` SKILL 은 워크플로우 본문 Step 1 에 정규 블록(`scripts/preflight-block.ts` 의 `CANONICAL_PREFLIGHT_BLOCK`)을 그대로 넣어요:
   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
   PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
   [ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
   ```
   load-time `!command` 주입 방식은 폐기됐어요 (§7 ADR-0013 참고).
2. **D1 TTY guard** — AskUserQuestion 을 쓰는 SKILL 은 `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경(`claude -p`/CI/headless)에서 질문을 건너뛰고 **안전 기본값**으로 진행해요. 기본값은 registry(아래)에서 읽어요.
3. **TodoWrite Step 0** — `multi-step: true` SKILL 은 워크플로우 맨 앞에서 `TodoWrite({ todos: [...] })` 로 진행 체크리스트를 띄워요. 끝나면 전부 `completed` 로 바꿔야 해요.
4. **AskUserQuestion fallback registry** — 모든 AskUserQuestion 은 `tests/fixtures/ask-defaults/registry.json` 에 `safe_default` + `rationale` 가 등록돼야 해요. 등록 없이 ship 하면 `tests/ux-ask-fallback-registry.test.ts` 가 fail. 예: deploy 의 "진행할까요?" → `safe_default: 미리보기만`(non-interactive 는 실배포 금지).

### 4.3 preflight — 한 번에 세상 상태 읽기

`preflight --json` (`preflight.rs:398`)은 네 가지 probe(CLI 버전 / `auth status` / 마지막 배포 캐시 / manifest)를 **병렬**(`thread::scope`)로 돌려 한 JSON 으로 합쳐요. `AXHUB_PREFLIGHT_PARALLEL=0` 이면 순차 실행(디버깅용).

주요 필드(`PreflightOutput`, `preflight.rs:212`):

| 필드 | 의미 |
|------|------|
| `cli_version` / `in_range` / `cli_too_old` / `cli_too_new` | CLI 버전과 지원 범위(0.17.3 ~ <1.0.0) |
| `cli_present` / `cli_state` | CLI 존재 + 상태(`ok`/`not_found`/`config_corrupted`/`runtime_error`) |
| `auth_ok` / `auth_error_code` / `user_email` / `expires_at` / `scopes` | 인증 상태 |
| `current_app` / `current_env` | 현재 앱 slug / profile |
| `last_deploy_id` / `last_deploy_status` | 마지막 배포 |
| `plugin_version` | helper 버전 |

SKILL 들은 이 JSON 을 `jq` 로 읽어 "인증 안 됐으면 auth 로, CLI 없으면 install-cli 로" 식 분기를 해요.

### 4.4 명시 확인 — preview-first destructive workflow

이전 helper-minted approval token 게이트는 제거됐어요. 현재 정본은 **SKILL 내부 preview card + AskUserQuestion 명시 승인 + 승인 직후 정확히 한 번의 destructive `axhub ... --json` 실행**이에요.

**메커니즘**:
- `deploy`, `apps`, `github`, `publish`, `tables` 같은 mutation SKILL 은 먼저 preflight/resolve/dry-run 으로 앱·환경·브랜치·커밋·예상 영향 5필드를 보여줘요.
- 사용자가 명시 승인하면 별도 토큰 mint 없이 바로 다음 단계에서 destructive 명령을 한 번 실행해요. 예: `axhub deploy create --app ... --json`.
- `미리보기만`/`취소`/headless D1 guard 경로에서는 mutation 을 실행하지 않아요. headless 는 dry-run/read-only 증거까지만 남겨요.
- PreToolUse 의 legacy approval-token 차단점은 없고, hard gate 는 품질 commit/push `commit-gate` 만 남아요.
- `axhub-helpers consent --enable/--disable` 은 품질 megaskill on/off 플래그(`quality-consent.json`)라 destructive approval flow 와 별개예요.

읽기 전용 명령은 항상 preview card 없이 통과하고, mutation 은 SKILL 이 approval context 를 같은 턴에서 직접 보존해 실행해요. ADR-0009 는 preview-first 정책의 역사적 배경이고, 현재 구현 정본은 각 mutation SKILL 의 Step 3/4예요.

### 4.5 인증(auth) — 헷갈리기 쉬운 두 레이어

`auth` 라는 단어가 코드베이스에서 **두 가지 다른 것**을 가리켜요. 섞으면 안 돼요.

**레이어 1 — 플러그인/helper 가 `axhub-api` 를 호출할 때**:
helper 크레이트에는 **쿠키 코드가 전혀 없어요**. bearer 토큰 해석 체인 하나만 있어요:
```
env AXHUB_TOKEN (PAT) → $XDG_CONFIG_HOME/axhub-plugin/token (파일) → OS keychain
```
브라우저/device-code 로그인 흐름은 **upstream `axhub` CLI** 가 담당하고, helper 는 `axhub auth login --browser` 를 invoke 하거나 CLI 가 저장한 토큰을 읽을 뿐이에요. exit code **65** 가 토큰 만료/미인증 신호예요.

**레이어 2 — 유저가 작성한 앱 코드가 catalog API 를 호출할 때**:
이게 `docs/plugin-developer-guide.md` 의 "인증 2모드"예요. `snippet` 서브커맨드가 환경을 감지해 둘 중 하나의 코드를 생성해요:
- **Mode A (SSO 쿠키)** — 코드가 axhub 안(`*.jocodingax.ai` 동일 출처)에서 돌 때. `fetch(..., { credentials: 'include' })` — 브라우저가 `_hub_access` 쿠키를 자동 전송. 수동 auth 헤더 금지.
- **Mode B (PAT)** — 로컬/CI/외부. `headers: { "X-Api-Key": env["AXHUB_PAT"] }`. 토큰은 keychain/env 에만, 소스 하드코딩 금지.

즉 레이어 1 은 *플러그인 도구의 인증*, 레이어 2 는 *유저 앱 런타임의 인증*이에요. 문서/코드에서 "PAT" 가 나오면 어느 레이어인지부터 확인해요.

### 4.6 exit code 분류 + 복구 라우팅

`classify-exit` 가 PostToolUse 에서 axhub CLI 의 exit code 를 잡아 한국어 공감 메시지로 바꿔요(`data/catalog.json` 카탈로그 조회). 여기서 중요한 구분:

| 코드 | helper 가 직접 emit? | 의미 |
|------|----------------------|------|
| 0 | ✅ | 성공 / fail-open no-op |
| 1 | ✅ | 최상위 에러 / 네트워크 transport |
| 64 | ✅ | 사용법 에러 / 인자 오류 / 배포 진행중(`64:validation.deployment_in_progress`) |
| 65 | ✅ | 인증/토큰 실패 (만료 포함) |
| 67 | ✅ | not found (앱 slug 등) |
| 124 / 127 | ✅ | probe 타임아웃 / spawn 실패 |
| 66 / 68 / 70 | ❌ | **카탈로그 키로만 존재** — upstream `axhub` CLI 가 내는 코드라 helper 는 lookup 만 함 |

> 이 표는 "helper 가 직접 내는 코드" 분류 관점이에요. `data/catalog.json` 에는 이 외에도 `2`(배포 진행중) 같은 watch-state 키가 더 있어요 — classify-exit 가 메시지를 찾을 때만 쓰는 값이에요.

복구 라우팅: exit 65 → `auth`, 배포 진행중(64) → `status`, slug 모호(67) → `apps` disambiguation, 빌드 실패 → `recover`/`logs`.

### 4.7 `.axhub/` AI 컨텍스트 + snippet

`sync` 서브커맨드가 유저 프로젝트에 `.axhub/` 를 만들어요:
- `AXHUB.md` — Claude 가 데이터 접근 코드를 짜기 전에 **가장 먼저 읽는** 규칙 문서(인증 모드, SQL 규칙, 응답 처리).
- `AXHUB_TARGET` — 현재 모드(`web-axhub`/`local-*`). PII 없어서 commit 가능.
- `catalog.json` — `allowed_columns`·PII 태그·리소스 경로 스냅샷. `tenant_id`/`user_email` 포함이라 **gitignore**.
- `spec/` — migrate planning 전용. 앱별 승인 target-state spec 과 `latest.json` 포인터를 저장해요. execution 전 approval 이 나기 전에는 `latest.json` 을 갱신하지 않아요.
- `plan/` — migrate planning 전용. run별 stage artifact, approval, receipt, ADR, latest-run pointer 를 저장해요. 기본 root 는 repo-local 이고, `.axhub-workspace` marker opt-in 이 있을 때만 workspace 공유 root 로 확장해요.
- migrate planning 승격 규칙: simple high-confidence 는 기존 simple flow, low confidence/모호함은 serial `spec_only`, hard-stop/복잡 조건은 `discover → planner → architect → critic → reviewer` full consensus 예요.
- full consensus 에서는 planner/architect/critic role-agent 결과를 `.axhub/plan/runs/<run_id>/stages/*.md` 와 meta 로 남겨요. reviewer stage 는 전용 task agent 가 없어서 read-only `executor` lane 으로 completeness / scope sanity check 를 남겨요. 최종에 `approval.json.state=pending_approval`, `run.json.state=pending_approval` 로 올린 뒤 멈춰요.
- 승인되면 helper 의 `migrate-approve` 가 `.axhub/spec/apps/<app_key>/latest.json` 을 승격하고 run/approval/spec 상태를 `approved` 로 올려요. approval 전에는 latest pointer 를 만들지 않아요.
- wave 병렬화는 full consensus 내부의 same-app 독립 unit 에서만 허용해요. planner → architect → critic → reviewer 순서 자체는 직렬이고, write target 충돌·cycle·app_key mismatch·independence proof 부족이면 즉시 serial fallback 이에요.
- 플러그인 전용 migrate planning agent prompt 는 `agents/axhub-migrate-{discoverer,planner,architect,critic,reviewer}.md` 로 따로 ship 해요. plugin runtime 이 이 md 파일을 직접 읽어도 같은 책임 분리와 출력 형식을 유지하게 해요.

### 4.8 부가 시스템 (품질·관측·UX)

본 배포 경로 옆에 돌아가는 보조 시스템들이에요. `.axhub-state` 같은 게 왜 생기는지 여기서 풀려요.

- **품질 자동모드 (v1.0 라인)** — 코드 50+줄 변경/테스트 실패 같은 신호에 **다음 턴**에 품질 SKILL(`axhub-review`/`axhub-debug` 등) 호출을 권하는 best-effort 리마인더예요. 상태는 `.axhub-state/quality.json`. 단, **`git commit`/`git push` 는 리뷰 안 거치면 `commit-gate` 가 PreToolUse `ask` 로 막는 hard gate** 예요. `tdd-inject`(Edit/Write 시 TDD 리마인더)·`karpathy-inject`(코딩 가이드라인)·`using-axhub-quality`(megaskill 컨텍스트)도 같은 라인. 전체 끄기: `AXHUB_DISABLE_TRIGGERS=1`, 리뷰 게이트만: `AXHUB_SKIP_REVIEW=1`.
- **telemetry/관측** — opt-in. `AXHUB_TELEMETRY=0` 으로 끄고, `usage.jsonl`·배포 이벤트 NDJSON·phase marker 를 로컬에 남겨요.
- **audit/privacy** — 라우팅 audit 는 **prompt 원문을 저장 안 하고 sha256 해시만** 남겨요(7일 회전, 외부전송 X, `AXHUB_NO_AUDIT=1` 로 off). 짧은 발화("deploy" 6바이트 등)의 해시는 익명성 보장 안 됨을 명시해요.
- **statusline** — Claude Code plugin manifest 가 `statusLine` 필드를 지원 안 해서, `settings.json` 에 직접 병합해요. ADR-0012 의 결정으로 **silent default-ON + dual-channel 공개**(install.sh 또는 SessionStart 중 먼저 발사) + **orphan stub**(플러그인 삭제 후에도 빈 출력 exit 0)으로 graceful 하게 처리해요. `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 로 거부.

---

## 5. e2e 플로우

### 5.1 "내 앱 배포해" 한마디의 전체 여정

```
사용자: "내 paydrop 앱 배포해"
   │
[Step 0] UserPromptSubmit → prompt-route (preflight + audit 주입; 라우팅은 아님)
   │      Claude Code 가 SKILL description 으로 deploy SKILL 을 native 매칭 (model: sonnet)
   ▼
[Step 0.5] TodoWrite — 진행 체크리스트 렌더 (배포 경로에 맞춰 동적 생성)
   │
[Step 1] in-body preflight — PREFLIGHT_JSON=$("$HELPER" preflight --json)
   │      (첫 실행이면 Claude Code 가 Bash 권한을 한 번 물어요 → "허용")
   │      auth_ok=false → /axhub:auth · cli_not_found → /axhub:install-cli · too_old → /axhub:upgrade
   │
[Step 1] deploy-prep — preflight+resolve+bootstrap-plan 병렬 1콜
   │      → { preflight, resolve{app_id,branch,commit_sha,eta...}, in_flight_deploy, quality_gate, exit_code }
   │
[Step 1.5] git 준비 — 커밋 없으면 AskUserQuestion "저장 지점 만들까요?" → git init/add/commit → resolve 재실행
   │
[Step 1.6] in-flight 감지 — 이미 배포 중이면 commit_sha 비교로 3-way 분기
   │        (같음=진행중 / 다름=타인배포 가능 / 미상=확인중) → 보기/새배포/취소
   │
[Step 1.7] status-first 게이트 (GitHub 연결 앱) — push 트리거 배포가 이미 돌면 자동 watch 로 점프
   │
[Step 2] 버전 체크 — too_old 중단, too_new 는 계속/업그레이드/안묻기 선택
   │
[Step 3] preview card (AskUserQuestion) — 앱/환경/브랜치/커밋/예상시간 5필드 → [네 배포 / 미리보기만 / 취소]
   │      "취소" = mutation 없음 · "미리보기만" = --dry-run
   │
[Step 3.5] token-gate — 토큰 신선도 폴링(최대 30s); exit 65 → Step 6 복구
   │
[Step 4] 승인 직후 deploy create
   │      axhub deploy create --app ... --json     (같은 승인 턴에서 정확히 한 번 실행)
   │      exit 0 → DEPLOY_ID 추출 → Step 5
   │      exit 64 in-progress → in-flight ID 가져와 watch · 64 기타 → 공감 메시지(재시도 안 함)
   │      exit 65 → Step 6 · exit 67 → slug disambiguation
   ▼
[Step 5] watch (~3분) — axhub deploy status --watch --json
   │      성공 → /axhub:open (브라우저) · 실패 → /axhub:recover or /axhub:logs
   ▼
[끝] TodoWrite 전부 completed
```

### 5.2 복구 플로우

| 트리거 | 라우팅 |
|--------|--------|
| exit 65 (토큰 만료) | `auth` — device code 발급 → 브라우저 승인 → `auth status` 5초 폴링(최대 5분) → `--resume-last` 로 재개 |
| in-flight 배포 충돌 (64) | `status` — 진행 중 배포 watch |
| slug 모호 (67) | `apps` — 앱 목록에서 disambiguation |
| 빌드 실패 | `recover` / `logs` — build log 표면화 |
| 배포 중 토큰 만료 | Step 3.5 token-gate 가 **mutation 전에** 잡아 `auth` 로 — 배포 생성 전에 차단 |

핵심은 **fail-safe 순서**예요: 위험한 작업(deploy create) 앞에 항상 게이트(preflight, token-gate, preview card)가 있어서, 인증·상태가 깨진 채로 mutation 이 일어나지 않아요.

---

## 6. 개발 워크플로우

### 6.1 로컬 셋업

```bash
# 0. 툴체인: Bun >=1.1, Rust (Cargo workspace — .tool-versions 참고)
bun install                 # repo 툴링 의존성 (jose/semver/zod + dev)

# 1. helper 빌드 (호스트 타깃)
bun run build               # → bin/axhub-helpers

# 2. smoke
bun run smoke               # build + version + help
bin/axhub-helpers version
# Windows PowerShell 에서는 `bin\axhub-helpers.exe version` 를 사용해요

# 3. Rust 테스트
bun run cargo:test          # cargo test --workspace
```

### 6.2 새 SKILL 추가 (직접 만들지 마요)

`mkdir skills/foo` 후 손으로 `SKILL.md` 를 쓰면 Phase 17/18 패턴(D1 guard / TodoWrite / in-body preflight / registry stub)이 빠져 CI 가 fail 해요. **반드시 scaffold**:

```bash
bun run skill:new my-skill                       # mutate-aware 기본값 (multi-step + needs-preflight)
bun run skill:new my-readonly --no-multi-step --no-preflight --model haiku   # 조회 전용

# 본문 채우고:
bun run skill:doctor          # 패턴 누락 colored 진단 (CI 는 --strict)
bun run lint:tone --strict    # 해요체 톤 (합니다/입니다/드립니다 금지)
bun run lint:keywords --check # nl-lexicon 트리거 베이스라인 잠금
bun test                      # 회귀
```

새 AskUserQuestion 을 넣었으면 `tests/fixtures/ask-defaults/registry.json` 에 `safe_default`+`rationale` 등록도 잊지 마요. 규칙 전문은 [`CLAUDE.md`](CLAUDE.md) / [`AGENTS.md`](AGENTS.md) "Skill Authoring" 섹션.

### 6.3 새 hook 추가

1. 핸들러 **첫 줄**에 `hook_safety::is_hook_disabled("name")` 호출 → disabled 면 허용 envelope + exit 0.
2. **fail-open**: 어떤 경로도 non-zero exit 금지. `unwrap()`/`panic!()` 금지. 실패는 `append_hook_error("name", &err)`.
3. shell wrapper(`hooks/<name>.sh`/`.ps1`)가 있으면 kill switch 를 동일하게 미러.
4. `docs/HOOKS.md` §1 표와 `tests/hooks-kill-switch.test.ts` 매트릭스에 등록.
5. `hooks/hooks.json` 에 이벤트 등록.

### 6.4 새 helper 서브커맨드 추가

`main.rs:run()` 의 `match` 에 arm 추가 → 핸들러 `fn cmd_xxx() -> anyhow::Result<i32>` 작성 → `i32` exit code 반환. hook 성격이면 `hook_output::*` 의 JSON envelope 빌더를 써요. `USAGE` 문자열도 갱신해요.

### 6.5 릴리스

[§3.5](#35-릴리스-하네스--2단계--cosign) 의 2단계 절차를 그대로 따라요.

### 6.6 Troubleshooting / 로컬 디버깅

hook-driven 플러그인의 1순위 질문 — **"hook 은 떴는데 아무 일도 안 일어났어요. 어디를 보죠?"** — 답:

```bash
# 1. hook 에러 로그부터 (모든 fail-open 실패가 여기 쌓여요)
cat "${XDG_STATE_HOME:-$HOME/.local/state}/axhub-plugin/hook-errors.jsonl"

# 2. hook 을 로컬에서 직접 재현 — 이벤트 JSON 을 stdin 으로
echo '{"prompt":"배포해"}' | bin/axhub-helpers prompt-route
echo '{"tool_response":{"exit_code":65,"stderr":"auth expired"}}' | bin/axhub-helpers classify-exit

# 3. preflight 가 뭘 보는지
bin/axhub-helpers preflight --json | jq .
AXHUB_PREFLIGHT_PARALLEL=0 bin/axhub-helpers preflight --json   # 순차(디버깅)

# 4. 특정 hook 만 끄고 격리
AXHUB_DISABLE_HOOK=prompt-route,commit-gate claude
AXHUB_DISABLE_HOOKS=1 claude                                    # 전부 끄기

# 5. 세션 상태 직접 보기
cat "${XDG_STATE_HOME:-$HOME/.local/state}/axhub-plugin/session-bundle.json"
ls "${XDG_STATE_HOME:-$HOME/.local/state}/axhub" 2>/dev/null || true

# 6. 라우팅 통계
bin/axhub-helpers routing-stats --since 7d
bin/axhub-helpers doctor --json | jq .
```

Windows PowerShell 에서는 같은 예제를 `bin\axhub-helpers.exe <subcommand>` 형태로
실행해요. stdin 예시는 `Get-Content .\tests\hook-fixtures\v0\sessionstart.json |
.\bin\axhub-helpers.exe session-start` 처럼 PowerShell 파이프를 사용해요.

자주 겪는 사용자 에러(토큰 만료/동시 배포/slug 모호/Windows fallback) 한국어 가이드는 [`docs/troubleshooting.ko.md`](docs/troubleshooting.ko.md).

---

## 7. 핵심 ADR (왜 이렇게 결정했나)

| ADR | 결정 | 핵심 이유 |
|-----|------|-----------|
| [0009](docs/adr/0009-free-form-preview-policy.md) | free-form preview 카드 허용 | 강제 지점은 prose 가 아니라 Rust preview 게이트. 풍부한 배포 식별 정보는 작은 option 스키마에 안 맞음 |
| [0010](docs/adr/0010-stderr-filter-graceful-degradation.md) | stderr 필터는 best-effort + Step 6 한국어 fallback | CLI stderr 포맷 drift 시 raw 노출돼도 한국어 공감 템플릿이 바로 뒤따라 UX 손상 제한 |
| [0011](docs/adr/0011-skill-preflight-permission-fallback.md) | **SUPERSEDED** (→ 0013) | "바깥 `node -e` 는 권한 게이트 안 걸린다"는 전제가 프로덕션에서 거짓 → 한국어 fallback 이 dead path |
| [0012](docs/adr/0012-statusline-autowire.md) | statusline silent default-ON + dual-channel 공개 + orphan stub | 마켓플레이스 설치는 install.sh 를 안 거침 → 단일 채널로는 누락 |
| [0013](docs/adr/0013-skill-preflight-in-body.md) | **in-body preflight** (load-time `!command` 주입 제거) | 문서화된 Claude Code Bash 기본 동작(권한 prompt)에만 의존하는 유일한 방식. `skill:doctor` 가 계약 강제 |

추가로 **env var taxonomy(§10.6)**: 모든 axhub env 는 `AXHUB_DISABLE_<scope>`(opt-out) / `AXHUB_ENABLE_<scope>`(opt-in) / `AXHUB_<scope>=<value>`(값) 세 polarity 규칙을 따라요. 그래서 legacy `DISABLE_AXHUB` 대신 `AXHUB_DISABLE_HOOKS` 가 canonical 이에요.

---

## 8. Phase 모델

코드·문서 곳곳에 나오는 **"Phase NN"** 은 calendar 스프린트도 버전 번호도 아니에요. **하나의 응집된 아키텍쳐 관심사로 묶인 작업 증분**이고, 보통 `ralplan` 합의 게이트(Planner→Architect→Critic 가 모두 APPROVE)를 통과한 뒤 한 개 이상의 semver 패치/마이너로 ship 돼요.

> Phase 라벨은 `CHANGELOG.md` 의 **버전 헤더가 아니라 narrative 본문**과 `PLAN.md` 에 살아요. 한 Phase 가 여러 버전에 걸치기도 해요.

코드 검증된 실제 anchor 몇 개:

| Phase | 무엇 | 흔적 |
|-------|------|------|
| Phase 1–4 | sh/ps1 절차를 Rust 서브커맨드로 흡수 | v0.9.0 narrative |
| Phase 17/18 | **SKILL 작성 계약** — scaffold·frontmatter·skill:doctor·registry·톤/키워드 lint | AGENTS.md "Phase 17/18 강제" |
| Phase 19 | 릴리스 자동화 — `release`/`release:tag`/`release:check` | AGENTS.md "Release Workflow" |
| Phase 25 | hook 안전 계약(fail-open·kill switch) + SKILL `model:` 라우팅 | `docs/HOOKS.md`, `hook_safety.rs` |
| Phase 26 | v1 품질 자동화 — 5 품질 SKILL + 3 agent + commit-gate | v0.7.0 narrative |
| Phase 27 | in-body preflight 채택 (ADR-0013) | v0.9.15 narrative |

---

## 9. 안전·신뢰

### 안전 가드 (요약)
- **preview confirmation 토큰** — destructive op 차단 (`CLAUDE_SESSION_ID` 바인딩, O_NOFOLLOW, symlink reject) — §4.4
- **CLI 경계 신뢰** — helper 는 더 이상 자체 HTTP/TLS 스택을 안 가져요. deploy/auth probe 는 캐노니컬 `axhub` CLI subprocess 를 통해서만 호출하고, TLS posture·프록시 설정·인증서 검증은 모두 CLI 가 담당해요 (`crates/axhub-helpers/src/axhub_cli.rs`). 예전 SPKI 핀(`security.tls_pin_failed`) + `AXHUB_ALLOW_PROXY=1` 우회는 PR #149 에서 제거됐어요.
- **exit 65 → 한국어 안내 + auth 흐름**
- **SessionStart preflight 진단** + fail-open hook
- **인증 2모드(레이어 2)**: 유저 앱 코드는 Mode A 쿠키 / Mode B PAT — §4.5 (helper 자체는 bearer 토큰 체인, 쿠키 안 씀)

### Trust & Uninstall
설치 중 수행하는 신뢰 이벤트를 투명 공개해요: ① 인증 토큰 저장(keychain/file) ② opt-in telemetry(`AXHUB_TELEMETRY=0` off) ③ macOS Gatekeeper quarantine 제거 ④ auth-refresh 백그라운드 ⑤ helper binary 자동 다운로드(GitHub release HTTPS) ⑥ `~/.claude/settings.json` statusLine 관리(타 plugin 설정 보존).

- statusLine 자동관리 거부: `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1`
- 기존 settings.json 치유(v0.6.2): `axhub-helpers settings-merge --migrate --yes` (git-tracked 면 경고만)
- dotfile sync(chezmoi/Dotbot) 사용자는 자동 수정으로 working tree 가 dirty 해질 수 있어 위 env 권장

### Statusline 보이게 하기
manifest 가 `statusLine` 를 지원 안 해서 `~/.claude/settings.json` 에 직접 활성화해야 해요. `/axhub:enable-statusline` → "복사해서 붙여 넣을래요" 선택 시 `settings-merge --apply` 가 자동 기록 → Claude Code 재시작. 지원: macOS/Linux/Windows(Git Bash/WSL/native PowerShell 5.1+).

---

## 10. 레퍼런스

### 전체 SKILL 표 (42)

`ms`=multi-step, `pf`=needs-preflight. 모든 SKILL 은 `/axhub:<이름>` 슬래시로 호출돼요 — 슬래시 칸의 별도 이름(`/axhub:login`·`/axhub:배포`)은 `commands/` 의 전용 alias 예요.

| SKILL | 트리거 예시 | 슬래시 | ms | pf | model |
|-------|-------------|--------|----|----|-------|
| `deploy` | "내 앱 배포해", "ship" | `/axhub:deploy` `/axhub:배포` | ✅ | ✅ | sonnet |
| `migrate` | "기존 앱 올려줘", "migrate this repo" | `/axhub:migrate` | ✅ | ✅ | sonnet |
| `status` | "방금 배포 어떻게 됐어" | `/axhub:status` | — | — | haiku |
| `logs` | "빌드 로그 보여줘" | `/axhub:logs` | — | — | haiku |
| `recover` | "방금 거 되돌려" | `/axhub:recover` | ✅ | ✅ | sonnet |
| `apps` | "내 앱 목록" | `/axhub:apps` | — | ✅ | sonnet |
| `auth` | "axhub 로그인해줘" | `/axhub:auth` `/axhub:login` | — | — | sonnet |
| `update` | "axhub CLI 새 버전 있어" | `/axhub:update` | ✅ | — | sonnet |
| `upgrade` | "플러그인 업그레이드" | `/axhub:upgrade` | ✅ | — | sonnet |
| `doctor` | "axhub 설치돼 있어" | `/axhub:doctor` | ✅ | — | haiku |
| `init` | "결제 앱 만들어줘" | `/axhub:init` | ✅ | — | sonnet |
| `onboarding` | "처음인데 셋업해줘" | `/axhub:onboarding` | ✅ | — | sonnet |
| `install-cli` | "axhub CLI 설치해줘" | `/axhub:install-cli` | ✅ | — | sonnet |
| `repair` | "PATH 고쳐줘" | `/axhub:repair` | ✅ | — | sonnet |
| `env` | "환경변수 뭐 있어" | `/axhub:env` | ✅ | ✅ | sonnet |
| `github` | "GitHub repo 연결해" | `/axhub:github` | ✅ | ✅ | sonnet |
| `open` | "결과 봐" | `/axhub:open` | — | — | haiku |
| `profile` | "회사 endpoint 바꿔" | `/axhub:profile` | ✅ | — | sonnet |
| `my-resources` | "내가 쓸 수 있는 리소스" | `/axhub:my-resources` | — | ✅ | sonnet |
| `data` | "데이터 카탈로그 검색" | `/axhub:data` | ✅ | ✅ | sonnet |
| `verify` | "진짜 배포됐는지 확인" | `/axhub:verify` | ✅ | ✅ | — |
| `trace` | "왜 실패했는지 추적" | `/axhub:trace` | ✅ | ✅ | sonnet |
| `clarify` | (모호 발화 fallback) | `/axhub:clarify` | — | — | haiku |
| `routing-stats` | "라우팅 통계" | `/axhub:routing-stats` | — | ✅ | haiku |
| `enable-statusline` | "statusline 켜줘" | `/axhub:enable-statusline` | — | — | haiku |
| `axhub-review` | "코드 리뷰해줘" | `/axhub:axhub-review` | ✅ | ✅ | sonnet |
| `axhub-debug` | "이거 디버그해줘" | `/axhub:axhub-debug` | ✅ | ✅ | sonnet |
| `axhub-diagnose` | (배포/테스트 실패 자동 진단) | `/axhub:axhub-diagnose` | ✅ | ✅ | sonnet |
| `axhub-ship` | "리뷰 통과했으니 배포" | `/axhub:axhub-ship` | ✅ | ✅ | sonnet |
| `axhub-tdd` | "TDD 로 짜줘" | `/axhub:axhub-tdd` | ✅ | ✅ | sonnet |
| `axhub-plan` | "개발 계획 세워줘" | `/axhub:axhub-plan` | ✅ | ✅ | sonnet |
| `using-axhub-quality` | (품질 모드 안내) | `/axhub:using-axhub-quality` | — | — | sonnet |
| `karpathy-guidelines` | (코딩 가이드 참조) | `/axhub:karpathy-guidelines` | — | — | sonnet |
| `app-lifecycle` | "앱 복제", "앱 일시정지", "앱 재개" | `/axhub:app-lifecycle` | ✅ | ✅ | sonnet |
| `rollback` | "이전 배포로 롤백" | `/axhub:rollback` | ✅ | ✅ | sonnet |
| `tables` | "테이블 만들", "컬럼 추가", "행 넣어" | `/axhub:tables` | ✅ | ✅ | sonnet |
| `connectors` | "DB 연결", "커넥터 추가", "postgres 연결" | `/axhub:connectors` | ✅ | ✅ | sonnet |
| `resources` | "리소스 이름 바꿔", "리소스 정리", "네임스페이스 만들" | `/axhub:resources` | ✅ | ✅ | sonnet |
| `publish` | "앱 공개", "마켓에 올려", "심사 제출" | `/axhub:publish` | ✅ | ✅ | sonnet |
| `browse` | "앱 둘러봐", "템플릿 뭐 있어", "공개 앱 찾아" | `/axhub:browse` | ✅ | ✅ | haiku |
| `team` | "팀원 초대", "초대 목록", "접근 권한" | `/axhub:team` | ✅ | ✅ | sonnet |
| `workspace` | "내 워크스페이스", "테넌트 목록" | `/axhub:workspace` | ✅ | ✅ | haiku |
| `inspect` | "매니페스트 확인", "axhub.yaml 검증", "설정 확인" | `/axhub:inspect` | ✅ | — | haiku |

> `model` 미선언(`verify`)도 `skill:doctor --strict` 통과해요(Phase 25.5a no-op 약속). 선언하면 `haiku|sonnet|opus` 중 하나여야 해요.

### 주요 환경변수

| 변수 | 효과 |
|------|------|
| `AXHUB_DISABLE_HOOKS=1` | 모든 hook off (canonical kill switch) |
| `AXHUB_DISABLE_HOOK=a,b` | 지정 hook 만 off (csv) |
| `DISABLE_AXHUB=1` | legacy alias (deprecated, 경고) |
| `AXHUB_DISABLE_TRIGGERS=1` | 품질 자동모드 전체 off |
| `AXHUB_SKIP_REVIEW=1` | commit/push 리뷰 게이트만 off |
| `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` | statusLine 자동관리 off |
| `AXHUB_DISABLE_MEGASKILL` / `AXHUB_DISABLE_KARPATHY` / `AXHUB_DISABLE_POSTCOMMIT` | 개별 품질 기능 off |
| `AXHUB_NO_AUDIT=1` | 라우팅 audit off |
| `AXHUB_TELEMETRY=0` | telemetry off |
| `AXHUB_TOKEN` | helper 인증 PAT (레이어 1) |
| `AXHUB_PAT` | 유저 앱 코드 Mode B PAT (레이어 2) |
| `AXHUB_ENDPOINT` | axhub-api endpoint override (캐노니컬 CLI 가 소비) |
| `AXHUB_BIN` | helper 가 사용할 `axhub` 실행 경로 override (기본 `axhub`) |
| `AXHUB_SKIP_AUTODOWNLOAD=1` | helper binary 자동 다운로드 off |
| `AXHUB_PREFLIGHT_PARALLEL=0` | preflight 순차 실행 (디버깅) |
| `CLAUDE_PLUGIN_ROOT` | 플러그인 루트 절대경로 (Claude Code 주입) |
| `CLAUDE_SESSION_ID` | Claude Code 세션 식별자 |

### 용어집

| 용어 | 뜻 |
|------|-----|
| **preflight** | CLI·인증·앱·환경 상태를 한 번에 읽는 probe (JSON) |
| **preview confirmation** | destructive 명령 전에 사용자가 앱·환경·브랜치·커밋·예상 영향을 확인하는 preview card + AskUserQuestion 단계 |
| **in-body preflight** | SKILL 워크플로우 본문에 넣는 정규 preflight bash 블록 (ADR-0013) |
| **D1 guard** | non-interactive 환경에서 AskUserQuestion 을 안전 기본값으로 대체 |
| **nl-lexicon** | SKILL `description` 의 자연어 트리거 어구 (라우팅 진실원천, baseline 잠금) |
| **megaskill** | SessionStart 가 주입하는 품질 자동모드 컨텍스트 |
| **helper-pick** | `CLAUDE_PLUGIN_ROOT/bin` → PATH 순으로 바이너리 찾는 fallback |
| **fail-open** | hook 이 어떤 실패에도 exit 0 으로 메인 흐름을 안 막는 계약 |
| **Phase NN** | scope 로 묶인 개발 증분 (ralplan 합의 게이트) |

### 더 읽을 문서

| 문서 | 용도 |
|------|------|
| [`docs/plugin-developer-guide.md`](docs/plugin-developer-guide.md) | plugin v1 설계 정본 (auth 2모드·`.axhub/`·snippet·catalog API·DoD) |
| [`docs/HOOKS.md`](docs/HOOKS.md) | hook 안전·fail-open 계약 정본 |
| [`docs/routing.md`](docs/routing.md) | NL 라우팅 + audit/privacy 상세 |
| [`docs/RELEASE.md`](docs/RELEASE.md) | 릴리스 절차 상세 |
| [`docs/adr/`](docs/adr/) | 아키텍쳐 결정 기록 (0009–0013) |
| [`AGENTS.md`](AGENTS.md) / [`CLAUDE.md`](CLAUDE.md) | AI 에이전트 작업 규칙 (skill authoring / release 계약) |
| [`docs/troubleshooting.ko.md`](docs/troubleshooting.ko.md) | 사용자 에러 한국어 가이드 |
| [`docs/org-admin-rollout.ko.md`](docs/org-admin-rollout.ko.md) | 조직 관리자 롤아웃 |
| [`docs/vibe-coder-quickstart.ko.md`](docs/vibe-coder-quickstart.ko.md) | 바이브코더 빠른 시작 |

---

## 11. 빠른 시작 (사용자용)

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add jocoding-ax-partners/axhub

# 2. 플러그인 설치 (macOS/Linux 첫 SessionStart 에서 OS/arch 맞는 helper 자동 다운로드)
/plugin install axhub@axhub
#  └─ 자동 다운로드 끄기: export AXHUB_SKIP_AUTODOWNLOAD=1
#  └─ Windows native 자동 SessionStart 는 platform-specific hook 검증 전까지 deferred 예요

# 3. 첫 인증
"axhub 로그인해줘"              # 또는 /axhub:login
#  └─ headless: AXHUB_TOKEN env 또는 token-import (PowerShell 은 $env:AXHUB_TOKEN / axhub-helpers.exe token-import)

# 4. 첫 배포
"내 paydrop 앱 배포해"
```

준비물: 최신 Claude Code · axhub SaaS 계정 + scope(회사 admin 발급). 플랫폼별: macOS / Linux 자동 셋업 / Windows native 는 명시적 PowerShell 설치·token-import·AXHUB_TOKEN 경로, Git Bash·WSL 은 POSIX fallback.

---

## 부록 A — axhub-helpers 심화 레퍼런스

`bin/axhub-helpers`(크레이트 `crates/axhub-helpers`)는 이 플러그인의 코어예요. 모든 hook 진입점과 SKILL 의 in-body preflight 가 이 바이너리 하나를 호출해요. 더 깊이 만지기 전에 알아둘 것들이에요.

### A.1 바이너리 구조와 디스패치

진입은 `src/main.rs` 예요.

```rust
fn main() {
    enable_utf8_console();              // Windows: 콘솔 코드페이지를 UTF-8(65001)로 — 한글 mojibake 방지
    std::process::exit(match run() {    // run() 이 돌려준 i32 가 곧 프로세스 exit code
        Ok(code) => code,
        Err(e) => { eprintln!("{e}"); 1 }
    });
}
```

`run()` (`main.rs:74`)은 `clap` 의존성이 있는데도 **손으로 짠 `match`** 로 디스패치해요: `args[0]` 을 서브커맨드로 보고, 나머지를 `Vec<String>` 으로 모아 핸들러로 넘겨요. 인자가 없으면 `USAGE` 출력 후 `64`, 모르는 서브커맨드도 `64` 예요. 각 핸들러 시그니처는 `fn cmd_xxx(...) -> anyhow::Result<i32>` 이고, 반환한 `i32` 가 그대로 exit code 가 돼요.

> Windows 콘솔 codepage 는 process-attached scope 라 바이너리 종료 시 함께 사라져요 — 부모 `cmd.exe` 세션에 영향 0. pipe redirect 시 `SetConsoleOutputCP` 가 0 을 반환해도 fail-open 으로 무시해요.

### A.2 전체 서브커맨드 카탈로그

`axhub-helpers help` 의 `USAGE` 에 선언된 것들을 분류했어요:

| 분류 | 서브커맨드 |
|------|-----------|
| **hook 진입점** | `session-start` · `prompt-route` · `classify-exit` · `commit-gate` |
| **인증·토큰** | `token-init [--json]` · `token-import [--json]` · `token-gate` · `auth-refresh-bg` · `path <token-file\|last-deploy-file\|state-dir>` |
| **배포 파이프라인** | `preflight` · `resolve` · `deploy-prep --intent <n>` · `list-deployments` · `verify --app-id <id>` · `trace --deploy-id <id>` · `bootstrap [...]` · `emit-deploy-complete` · `mark <phase>` |
| **AI 컨텍스트** | `sync [--target auto]` · `snippet --mode A\|B --language <l> ...` |
| **UX·설정** | `settings-merge --apply\|--dry-run` · `autowire-statusline --scope <s>` · `orphan-stub --install` · `statusline` · `config get/set` |
| **진단·관측** | `doctor [--json]` · `routing-stats [--since <d>]` · `routing-dashboard [--html]` · `cleanup-audit [--all]` · `audit-clarify` · `diagnose hitl` |
| **유틸** | `redact` · `post-install` · `version [--quiet]` · `help` |

> **정직한 디테일**: `USAGE` 문자열은 일부 hook 서브커맨드를 안 적어놨어요. `hooks.json` 이 실제로 호출하지만 `USAGE` 에 없는 것들 — `commit-gate` · `tdd-inject` · `test-classifier` · `state-update` · `karpathy-inject`, 그리고 품질 megaskill on/off 플래그인 `consent` — 도 `run()` 의 `match` 에는 존재해요. `main.rs` 를 읽을 땐 `USAGE` 가 아니라 `match` arm 이 authoritative 예요.

### A.3 hook 호출 계약 (stdin → stdout)

hook 서브커맨드는 모두 같은 계약을 따라요:

- **입력**: Claude Code 가 hook 이벤트 JSON 을 **stdin** 으로 넘겨요 (예: `{"prompt": "..."}`, `{"tool_name":"Bash","tool_input":{"command":"..."}}`, `{"tool_response":{"exit_code":65}}`).
- **출력**: **stdout** 에 JSON envelope 한 개를 찍고 **exit 0**. Claude Code 가 그걸 구조화 출력으로 읽어요.
- **무출력 no-op**: 주입할 게 없으면 `{}` (빈 객체)를 찍어요.

envelope 빌더는 `src/hook_output.rs` 에 모여 있어요:

| 빌더 | 용도 | 핵심 필드 |
|------|------|-----------|
| `session_start_context` / `user_prompt_context` / `pre_tool_use_context` / `post_tool_use_context` | 컨텍스트 주입 | `hookSpecificOutput.additionalContext` (에이전트용, 영문 태그 블록) |
| `pre_tool_use_allow` / `pre_tool_use_ask` / `pre_tool_use_deny` | 권한 결정 | `hookSpecificOutput.permissionDecision` (+ `permissionDecisionReason`) |

`systemMessage`(사용자에게 보이는 한국어 prose)와 `additionalContext`(에이전트만 보는 영문 구조 컨텍스트)는 **다른 채널**이에요. 한 hook 이 둘을 동시에 낼 수도 있어요(`session-start` 가 그래요). `additionalContext` 의 태그 블록은 `lint:hook-inject` 가 `Observed:`/`Suggested:`/`Skip:` 토큰과 토큰 예산을 강제해요:

```
<axhub-preflight-status>
[axhub hook | session preflight]
Observed: axhub CLI v0.17.4 healthy.
Suggested: no action required.
Skip: AXHUB_DISABLE_HOOK=prompt-route
</axhub-preflight-status>
```

직접 재현하려면 이벤트 JSON 을 stdin 으로 파이프해요 — 구체적 예시는 [§6.6 Troubleshooting](#66-troubleshooting--로컬-디버깅) 에 있어요.

### A.4 전체 모듈 맵

`src/lib.rs` 에 선언된 모듈과 책임이에요:

| 모듈 | 책임 |
|------|------|
| `preflight` | CLI/인증/앱/환경 4-probe 병렬 → `PreflightOutput` |
| `hook_safety` | `is_hook_disabled` (kill switch) + `append_hook_error` (fail-open 로깅) |
| `keychain` / `keychain_windows` | OS 키체인 토큰 읽기 (macOS `security` / Linux `secret-tool` / Windows PowerShell) |
| `catalog` | exit-code → 한국어 공감 메시지 (build.rs 가 `data/catalog.json` 임베드) |
| `list_deployments` | 캐노니컬 `axhub deploy list --json` CLI wrapper. 인자 검증(`validate_app_ref`) + stderr 마스킹(`redact`) 후 envelope 파싱 |
| `resolve` / `deploy_prep` / `bootstrap` | 앱/배포 컨텍스트 해석, 병렬 preflight+resolve, 앱 부트스트랩 |
| `verify_helper` / `trace_helper` / `recovery_scan` | 라이브 배포 verdict, 실패 trace, 중단 배포 분류 |
| `audit` / `audit_ledger` / `event_log` / `atomic_jsonl` | 라우팅 audit(해시), 진단 ledger, 배포 이벤트 NDJSON, atomic JSONL |
| `telemetry` / `observability` | phase marker, 배포-complete 봉투, NDJSON 이벤트 |
| `quality_state` / `quality_gate` / `commit_gate` / `test_classifier` / `tdd_inject` / `karpathy_inject` | v1 품질 자동모드 |
| `statusline` / `autowire` / `orphan_stub` / `settings_merge` | statusLine 렌더·자동설정·고아 stub·settings.json 병합 |
| `snippet` / `sync` | 유저 앱 connector 코드 생성, `.axhub/` 동기화 |
| `runtime_paths` / `config` / `redact` / `humanize` / `spawn` / `session_bundle` / `hook_output` | XDG state/runtime 경로, 유저 config, 비밀 redaction, 한국어 시간 표기, 서브프로세스, 세션 번들, hook envelope |
| `diagnose/*` | 자동 진단 시스템 (hitl/probe/fix/hypothesis/postmortem/recurrence) |

### A.5 외부 의존성과 빌드 특이점

`Cargo.toml` 의 주요 크레이트와 쓰임새:

| 크레이트 | 쓰임 |
|----------|------|
| `axhub_cli` + `cli_envelope` (in-crate) | `axhub --json …` subprocess 실행 + JSON envelope 언래핑. PR #149 에서 reqwest/rustls/webpki-roots/x509-parser 의존성을 대체함 |
| `hmac` + `sha2` + `base64` | 관측 이벤트 해시, audit 해시, keychain blob 디코딩 |
| `getrandom` | 관측용 salt 난수 생성 |
| `clap` + `shlex` | 인자/명령 파싱 보조 |
| `tokio` + `crossterm` | 비동기 런타임 + 터미널 제어 |
| `serde` + `serde_json` | 모든 JSON I/O |
| `fslock` + `libc` | 파일 락 + `O_NOFOLLOW` 등 POSIX |
| `unicode-normalization` + `chrono` + `semver` + `uuid` | 한글 정규화·시간·버전·ID |
| `windows-sys` (cfg windows) | 콘솔 codepage / 자격증명 |

빌드 특이점:
- `build.rs` 가 `data/catalog.json` 을 읽어 `catalog_generated.rs`(`CATALOG_JSON`)를 만들고, `catalog.rs` 가 `include!` 해요 — 공감 카탈로그는 **컴파일 타임 임베드**라 런타임 파일 의존이 없어요.
- `#[cfg(coverage)]` 빌드는 `list_deployments` 의 라이브 소켓 검증을 stub 으로 대체해 커버리지 측정 시 네트워크를 끊어요.
- 릴리스 바이너리는 5개 cross-arch(§3.2)이고 Windows 는 static CRT 링크(no `vcruntime140`/`msvcp140`)를 CI 가 dumpbin 으로 검증해요.

### A.6 누가 helper 를 호출하나

| 호출자 | 경로 |
|--------|------|
| Claude Code hook | `hooks.json` → `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers <hook 서브커맨드>` |
| SKILL 워크플로우 | 본문 bash 의 in-body preflight + `deploy-prep`/`preview approval`/`token-gate` 등 (helper-pick fallback) |
| 설치 스크립트 | `bin/install.sh`/`.ps1` 가 `post-install`·`token-import` 호출 |
| shell wrapper | `session-start.sh`/`.ps1` 가 binary 자동설치·warmup 후 `session-start` 로 `exec` |

---

## 라이선스

MIT — [`LICENSE`](LICENSE).

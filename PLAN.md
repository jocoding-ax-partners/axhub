<!-- /autoplan restore point: /Users/wongil/Desktop/work/jocoding/axhub/.autoplan/PLAN.restore-20260423-172529.md -->
# axhub Claude Code Plugin — 초안 계획서

> 한 줄: `ax-hub-cli`(v0.1.0 GA)를 Claude Code가 자연어로 90% 이상, 보조적으로 슬래시 커맨드로 안전하게 사용해 axhub 앱을 배포·관리하도록 만드는 플러그인.

작성일: 2026-04-23 · 최근 업데이트: 2026-04-28 · 대상 contract: ax-hub-cli **v0.1.0** GA · 현재 릴리즈: **axhub plugin v0.1.22** · 작업 디렉토리: `/Users/wongil/Desktop/work/jocoding/axhub`

---

## 0. 현재 구현/릴리즈 스냅샷 (2026-04-28, v0.1.22)

> 이 섹션은 PLAN.md가 stale open-work list처럼 보이지 않도록, 2026-04-28 기준으로 실제 머지·릴리즈된 구현 범위를 고정한다. 아래 항목은 모두 `main`에 머지되었고, v0.1.21 baseline 이후 SessionStart hotfix는 `v0.1.22` patch release로 배포한다.

| Status | 범위 | PR / commit | 구현·검증 근거 |
|---|---|---:|---|
| **SHIPPED** | v0.1.20 exhaustive review bugfix baseline: consent-token safety, release automation drift, skill/docs contract drift | PR #3, tag `v0.1.20` | `tests/consent.test.ts`, `tests/release-config.test.ts`, `tests/manifest.test.ts`, `bun run release:check`, staging E2E |
| **SHIPPED** | Phase 1 command-surface reconciliation: canceled plugin-server/MCP scope가 active plan으로 되살아나지 않도록 PLAN/commands 정리 | PR #4, merge `5227f94` | `tests/plan-consistency.test.ts`, `tests/manifest.test.ts`, `commands/help.md`, `commands/배포.md` |
| **SHIPPED** | Phase 2 corpus runner replay: placeholder runner 대신 committed fixture를 replay/score 가능하게 전환 | PR #5, merge `d38f248` | `tests/run-corpus.sh`, `tests/run-corpus.test.ts`, `tests/README.md` |
| **SHIPPED** | Phase 3 SessionStart preflight: 시작 시 axhub 설치/버전/auth/profile 진단을 실제 메시지로 노출 | PR #6, merge `4cf3baf` | `src/axhub-helpers/index.ts`, `tests/session-start.test.ts` |
| **SHIPPED** | Phase 4 hook latency benchmark: impossible 5ms gate 제거, helper hot path p95 50ms gate를 측정 가능하게 고정 | PR #7, merge `f944fdf` | `scripts/benchmark-hooks.ts`, `tests/hook-latency.test.ts`, `package.json#bench:hooks` |
| **SHIPPED** | Phase 5 supply-chain/release plan sync: 현재 signed Bun helper release artifact와 PLAN/docs를 일치 | PR #8, merge `6eb8779` | `docs/RELEASE.md`, `.github/workflows/release.yml`, `tests/release-config.test.ts`, `bun run release:check` |
| **SHIPPED** | Phase 6 recover guidance sync: recover guidance를 shipped forward-fix flow로 문서화 | PR #9, merge `cc8d487` | `docs/troubleshooting.ko.md`, `tests/manifest.test.ts`, `tests/plan-consistency.test.ts` |
| **SHIPPED** | Phase 7 hub-api TLS pinning: deployment-list fallback이 bearer token 전송 전 hub-api SPKI pin을 검증 | PR #10, merge `ed67fb9` | `src/axhub-helpers/list-deployments.ts`, `tests/list-deployments.test.ts` |
| **SHIPPED** | Phase 8 PLAN checklist ledger: best-practices checklist를 unchecked TODO가 아닌 evidence ledger로 전환 | PR #11, merge `6364e66` | §16.7, `tests/plan-consistency.test.ts`, `bun run skill:doctor --strict` |
| **SHIPPED** | Phase 9 current layout/schema sync: PLAN의 repo layout, plugin schema, package version snippet을 실제 구현과 동기화 | PR #12, merge `9fd6c09` | §16.2/§16.12, `tests/plan-consistency.test.ts`, `tests/manifest.test.ts` |
| **SHIPPED** | Phase 21 release cut: PR #4–#12 누적분을 `v0.1.21`로 bump/tag/release | commit `75418a3`, tag `v0.1.21` | GitHub Release `v0.1.21`, release workflow `25028614673`, `scripts/release/verify-release.sh v0.1.21` |
| **SHIPPED** | Phase 22 SessionStart startup hotfix: non-Windows 호스트에서 universal `hooks.json`가 PowerShell hook을 실행해 startup error를 노출하던 문제 수정 | commit `a90edd7`, tag `v0.1.22` | `tests/manifest.test.ts`, `tests/smoke-windows-vm-checklist.ps1`, `docs/pilot/*`, `bun test`, `bun run release:check` |

**v0.1.22 검증 기록**

- `bun test` → 545 pass / 5 skip / 0 fail.
- `bunx tsc --noEmit` → pass.
- `bun run lint:tone --strict` → 0 error / 0 warning.
- `bun run lint:keywords --check` → OK.
- `bun run skill:doctor --strict` → pass.
- `bun run release:check` → 5 cross-arch binaries rebuilt/checked at `0.1.21`.
- `AXHUB_E2E_STAGING_ENDPOINT=https://hub-api.jocodingax.ai bun run test:e2e` → 4 pass / 1 skip / 0 fail.
- GitHub Actions release workflow `25028614673` → success; 21 release assets uploaded.
- `bash scripts/release/verify-release.sh v0.1.21` → manifest cosign verification + 5 binary signature/checksum checks all OK.
- v0.1.22 hotfix pre-release gate: `bun test`, `bash tests/auto-download.test.sh`, `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun run skill:doctor --strict`, `bun run release:check`, `git diff --check`.

**남은 범위 정리**

- 이번 라운드에서 PLAN.md의 stale/mismatch/open-list 성격 항목은 닫혔다. 새 구현 TODO를 추가하려면 반드시 test/script gate 또는 manual evidence row를 같이 추가한다.
- `tests/run-corpus.sh`의 full live re-curation은 의도적으로 explicit `--fixture`가 필요하다. 자동으로 fabricated result를 만들지 않는 것이 현재 안전장치다.
- §13.3의 최소 2개 고객사 사용자 검증, deferred outside M0–M6 항목(자동 rollback, org-admin audit log skill, multi-tenant marketplace policy, web dashboard)은 코드 미구현이 아니라 별도 제품/운영 단계로 남긴다.
- GitHub Actions Node.js 20 deprecation 경고는 v0.1.21/v0.1.22 릴리즈 블로커가 아니었지만, 다음 release infra 라운드에서 runner/action Node 24 대응으로 추적한다.

---

## 1. Goal (revised after Phase 1)

> 원문: "내가 목표하는 것은 https://github.com/jocoding-ax-partners/ax-hub-cli 이 cli를 claude code가 자유롭게 잘 사용해서 자연어(90퍼센트 이상)나 슬래시 커맨드를 이용해서 앱을 배포하고 관리하는 플러그인을 만들고 싶어"
>
> 추가 컨텍스트 (Phase 1 premise gate): "바이브코딩하는 사람들의 hub를 만드는거야. 바이브코딩하면 앱들이 많이 만들어지는데 이런걸 vercel 이런데다 올리면 너무 파편화 되어 있잖아. 이런걸 한곳에 모아서 관리하고 싶은거야. 회사 같은데서. 그래서 우리가 만든 서비스를 이용하는 회사의 바이브코더들이 사용할 플러그인이야"

**진짜 정체**: axhub (바이브코더용 통합 앱 hub) SaaS를 도입한 회사의 **바이브코더 직원들**이, Claude Code에서 자연어로 자기 앱을 **안전하게** 배포·관리·관찰할 수 있게 하는 B2B end-user-facing 플러그인.

**중요한 architectural decision (사용자 명시 — Phase 6 후, 2회 정정)**: 

- app-hub-backend(`/Users/wongil/Desktop/work/jocoding/app-hub-backend`)는 현재 **MCP 서버**. 그 중 **CLI로 대체할 수 있는 기능만 ax-hub-cli로 마이그레이션** 중. MCP 자체를 폐기하는 것은 아님 — CLI로 못 옮기는 MCP 기능은 backend에 유지될 수 있음.
- 본 **플러그인**의 architectural rule: **항상 ax-hub-cli만 호출**. plugin 자체가 MCP 서버를 expose하지 않음. MCP를 직접 호출하지도 않음.
- Phase 5 §16.6 / Phase 6 §16.15에서 추가했던 "**M7 plugin MCP server (cross-agent portability)**" 계획 = **CANCELLED** (사용자 의도와 불일치, cross-agent는 사용자 우선순위 X).
- Codex/Cursor 등 다른 agent용 cross-agent layer가 미래에 필요하면 → ax-hub-cli 자체가 모든 agent 공통 surface (CLI는 본질적으로 cross-agent). 별도 plugin MCP layer 불필요.

요구 분해 (revised):
- **End user = vibe coder**, not 시니어 개발자. LLM으로 앱 만든 비기술/주니어. 배포는 무서운 작업. 트러스트 모델이 1순위.
- **자연어 우선** — 슬래시 사용 없이 "내 앱 어떻게 됐어", "방금 배포한 거 살아 있어?" 같은 자연 발화에 반응. 단, 메트릭은 trigger 비율이 아닌 trusted task completion + unsafe-trigger 0%.
- **슬래시는 escape hatch** — 명시적 컨트롤이 필요한 사용자용.
- **B2B 배포 channel** — 각 고객사가 marketplace에서 self-install. plugin = axhub 영업 자료의 일부.
- **Cross-agent portability (superseded)** — vibe coder들이 여러 agent를 쓸 수는 있지만, Phase 6.5 사용자 정정(rows 61–64) 이후 plugin 자체의 MCP/cross-agent 계층은 **CANCELLED**. 공통 surface는 별도 plugin layer가 아니라 `ax-hub-cli` 자체다.

## 1.5 Personas (new)

| Persona | 누구 | 핵심 요구 | 핵심 두려움 |
|---|---|---|---|
| **Vibe Coder (주)** | LLM으로 앱 만드는 비기술/주니어. axhub 도입 회사 직원. | "내 앱이 살아 있는지" / "방금 만든 거 띄워줘" / 무서움 없이 배포 | 잘못 배포해서 prod 망가짐 / 무엇이 문제인지 모름 / 토큰 만료 같은 것에 막힘 |
| **Senior dev (부)** | vibe coder 배포를 리뷰/감독. 같은 plugin을 더 detailed하게 사용. | apps list 전체 view / 다른 사람 배포 status / API catalog 검색 | 권한 leak / vibe coder가 prod 망가뜨림 |
| **Org admin (관찰자)** | axhub 도입 회사의 IT/보안. plugin 설치 정책 관리. | scope 범위 제어 / 토큰 lifecycle / audit log | 무권한 행동 / 보안 incident |

**1차 design driver = Vibe Coder.** 부/관찰자는 Phase 2 이후 검토.

## 2. Premises (revised after Phase 1, P1–P10)

| # | Premise | 근거 | 위험 / 검증 방법 |
|---|---|---|---|
| P1 | 타겟 contract = ax-hub-cli **v0.1.0** GA. v0.1.1 backlog 항목은 land 시점에 plugin minor bump. | CHANGELOG.md, release-notes/v0.1.0.md | v0.1.x release 마다 transcript 회귀 |
| P2 | 자연어 우선 ≠ 90% trigger 비율. **trusted task completion + unsafe-trigger 0%**가 진짜 게이트. | C2 (Phase 1 CEO 합의), 사용자 confirm "대체" | fixed corpus n≥100 + 20 negative phrases (한·영) |
| P3 (revised) | Plugin의 가치 3가지: (a) 한국어 우선 NL 트리거 표면, (b) destructive ops에 대한 opinionated safe defaults, (c) baseline 대비 measurable delta. 측정 안 되면 plugin 존재 의미 없음. | C5 (CEO 합의), Phase 1 reframe | M1.5 GO/KILL gate (docs-only baseline 대비 정량 비교) |
| P4 (revised) | **Multi-user / multi-machine reality** — 회사 = 여러 vibe coder, 각자 머신. cache는 cold start 가정. profile 명시 필수. | 사용자 confirm "회사의 바이브코더들" | profile 안 보이는 destructive op 차단 |
| P5 (revised) | Endpoint는 **회사별 multi-profile** (production은 default, 회사가 필요 시 staging 분리). `AXHUB_PROFILE` 항상 노출. | CLI flags.md (`AXHUB_PROFILE` 존재) | profile mis-targeting test |
| P6 (new) | 사용자 = **단일 회사 내 10–500명 vibe coders + senior 일부**. <10이면 docs로 충분, >1000이면 web dashboard 우선. | 사용자 컨텍스트 (B2B 회사 도입) | 첫 고객사 N 카운트로 검증 |
| P7 (new) | Destructive 행동 (deploy create, deploy logs --follow kill, update apply --force) **항상 명시적 사용자 승인 필요**. fuzzy NL trigger만으로 자동 실행 X. | C3 (CEO 합의) | unsafe-trigger 측정 + AskUserQuestion 패턴 |
| P8 (new) | Plugin은 **ax-hub-cli와 동일 maintainer + 동일 release cadence**. 별도 owner 분리 시 90일 내 abandonware. | jocoding-ax-partners org | CODEOWNERS + CI matrix |
| P9 (superseded) | **Plugin cross-agent/MCP 계층은 scope 제외** — Phase 6.5 rows 61–64가 Phase 1/5/6의 M7 결정을 취소. Claude Code plugin은 항상 `ax-hub-cli`만 호출하고, 다른 agent도 필요 시 CLI를 직접 사용한다. | 사용자 2회 정정 ("plugin이 MCP를 쓰는게 아니라 cli를 쓰는거야") | PLAN active scope에서 M7/MCP placeholder가 다시 살아나지 않는지 문서/테스트로 검증 |
| P10 (new) | **Docs-only baseline 측정이 plugin 존재의 GO/KILL gate.** baseline ≥X% 능가 못하면 plugin 보류, docs로 ship. | C1 (CEO 합의) | M1.5에서 corpus 실측 |

→ **모든 premise는 §11 milestones에서 falsifiable check로 연결됨.**

## 3. Inputs (검증된 사실)

### 3.1 axhub v0.1.0 명령 매트릭스

| Group | Sub | 핵심 플래그 | Agent 핵심 |
|---|---|---|---|
| auth | login | (OAuth Device Flow, 브라우저 자동 열기) | exit 65 → 자동 트리거 |
| auth | logout | — | 토큰 제거 |
| auth | status | `--json` | scopes/expiry 확인, 사전 게이팅 |
| apps | list | `--json --per-page=N --all --slug-prefix` | slug 모호성 시 numeric `--app <id>` fallback |
| apis | list | `--json --query --app-id` | 카탈로그 검색 |
| deploy | create | `--app <slug\|id> --branch --commit --dry-run [--idempotency-key]` | `Idempotency-Key: <uuid>` 자동, **단일 실행** (W7 backend honour 전) |
| deploy | status | `<id> --app <slug\|id> --watch --json` | 정수 enum 0..5 → exit code, NDJSON tick 스트림 |
| deploy | logs | `<id> --app <slug\|id> --follow --source build\|pod --json` | SSE + `Last-Event-ID` 1회 재개, `eof:true` sentinel |
| update | check | `--json` | exit 0(no update) / 1(update) / 2(disabled), 24h 캐시 |
| update | apply | `--yes --force --json` | cosign opt-in (`AXHUB_REQUIRE_COSIGN=1`), homebrew/scoop 안내 분기 |

### 3.2 Exit code 계약

| Exit | Class | 자동 복구 |
|---|---|---|
| 0 | success | 다음 단계로 진행 |
| 1 | transport / unclassified | 읽기는 1회 backoff 재시도, mutation은 재시도 금지 |
| 2 | (deploy status 한정) in-progress / (update check 한정) disabled | `--watch` 또는 폴링 |
| 64 | validation/usage | **never retry**. 에이전트가 입력을 바꿔야 함 |
| 64 + `validation.deployment_in_progress` | concurrent deploy guard | `deploy status --watch`로 기존 배포 완료 대기 후 재시도 |
| 64 + `validation.app_ambiguous` | slug 중복 | numeric `--app <id>` 사용 |
| 64 + `validation.app_list_truncated` | >100 apps | numeric `--app <id>` 사용 |
| 65 | auth required/expired | `axhub auth login` 자동 트리거 |
| 66 | scope insufficient | 사람 개입 필요 (token 발급자가 scope 부여) |
| 66 + `update.cosign_verification_failed` | 공급망 검증 실패 | hard stop |
| 66 + `scope.downgrade_blocked` | 다운그레이드 시도 | 사용자 명시 `--force` 없으면 hard stop |
| 67 | resource not found | id/slug 재확인, 재시도 금지 |
| 68 | rate limited | `Retry-After` 또는 지수 backoff |

### 3.3 환경 변수

```
AXHUB_AGENT=1              # = --json + --no-input + ANSI strip
AXHUB_NO_INPUT=1           # CI/non-TTY
AXHUB_JSON=1               # 모든 명령에 --json
AXHUB_DISABLE_AUTOUPDATE=1 # CI/airgapped
AXHUB_REQUIRE_COSIGN=1     # update apply 시 cosign 강제
AXHUB_WATCH_INTERVAL=5s    # deploy status --watch 폴링 (1s..30s clamp)
AXHUB_WATCH_TIMEOUT=2m
AXHUB_PROFILE=<name>
AXHUB_ENDPOINT=<url>
AXHUB_TOKEN_FILE=<path>
AXHUB_CONFIG=<path>
AXHUB_TIMEOUT=30s
```

### 3.4 로컬 상태

- `~/.config/axhub/config.yaml` — profile/endpoint
- `~/.config/axhub/deployments.json` — `(deployment_id → app_id)` 캐시 → **같은 머신에서 후속 status/logs 호출 시 `--app` 생략 가능**
- `~/.config/axhub/.update.cache.json` — 24h update check 캐시
- OS keychain (또는 `--token-file` 폴백, 0600)

### 3.5 검증된 골든 패스 (실제 Claude Code transcript, 2026-04-23)

```bash
axhub auth status --json | jq -e .
axhub apps list --json | jq "[.[0:5] | .[] | {slug, status}]"
axhub apis list --json | jq "[.[0:3]]"
AXHUB_DISABLE_AUTOUPDATE=1 axhub update check --json
axhub deploy status dep_363 --app app-3 --json
# (deploy create + logs --follow는 prod 안전상 transcript에서 의도적 제외)
```

## 4. Architecture

```
사용자 발화 ("paydrop 배포해줘")
        │
        ▼
┌─────────────────────────────────┐
│ Claude Code (Opus / Sonnet)     │
│ + axhub plugin                  │
│                                 │
│ 1) UserPromptSubmit hook        │
│    - axhub 의도 키워드 감지     │
│    - 컨텍스트 로드 (현재 앱?)   │
│                                 │
│ 2) Skill 자동 invocation        │
│    description 매칭으로         │
│    deploy/status/logs 등 트리거 │
│                                 │
│ 3) SKILL.md 인스트럭션 →       │
│    Bash 도구로 axhub 호출       │
│    (--json 강제, AXHUB_AGENT=1) │
│                                 │
│ 4) PostToolUse(Bash) hook       │
│    - exit code 분류             │
│    - 65 → auth login 안내       │
│    - 64+in_progress → status    │
│    - 67 → 입력 검증 안내        │
│    - 68 → backoff 안내          │
└─────────────────────────────────┘
        │
        ▼
   axhub CLI binary
   (system PATH 또는 plugin bin/axhub-shim)
        │
        ▼
   https://hub-api.jocodingax.ai
```

**핵심**: 플러그인은 **얇은 routing layer**다. 비즈니스 로직은 모두 CLI에 있고, 플러그인은 (1) 자연어 인텐트 → 명령어 매핑, (2) 안전한 기본값 강제, (3) exit code 기반 자동 복구 안내만 담당한다.

## 5. Plugin layout

```
axhub/                          # plugin root = 현재 디렉토리
├── .claude-plugin/
│   ├── plugin.json             # name="axhub", version="0.1.0", description
│   └── marketplace.json        # 단일 플러그인 마켓플레이스 (jax-plugin-cc 패턴)
├── README.md                   # 한국어 사용 가이드
├── CHANGELOG.md
├── LICENSE                     # MIT? Proprietary? → §Phase1 premise gate에서 확인
├── settings.json               # (선택) 기본 설정
├── skills/                     # NL 자동 트리거의 핵심
│   ├── deploy/
│   │   └── SKILL.md            # "배포", "deploy", "ship", "푸시한 거 올려" 등
│   ├── status/
│   │   └── SKILL.md            # "배포 상태", "어떻게 됐어", "지금 진행 중인 거"
│   ├── logs/
│   │   └── SKILL.md            # "로그 보여줘", "빌드 로그", "실패 원인"
│   ├── apps/
│   │   └── SKILL.md            # "내 앱 목록", "어떤 앱들 있어"
│   ├── apis/
│   │   └── SKILL.md            # "API 카탈로그", "어떤 API 쓸 수 있어"
│   ├── auth/
│   │   └── SKILL.md            # "로그인", "권한 확인", "scope"
│   ├── update/
│   │   └── SKILL.md            # "axhub 업데이트", "버전 확인"
│   └── doctor/
│       └── SKILL.md            # "axhub 설치 확인", "환경 점검"
├── commands/                   # 명시적 슬래시 트리거
│   ├── deploy.md               # /axhub:deploy
│   ├── status.md               # /axhub:status
│   ├── logs.md                 # /axhub:logs
│   ├── apps.md                 # /axhub:apps
│   ├── apis.md                 # /axhub:apis
│   ├── login.md                # /axhub:login
│   ├── update.md               # /axhub:update
│   └── doctor.md               # /axhub:doctor
└── hooks/
    ├── hooks.json              # 이벤트 라우팅
    └── scripts/
        ├── ensure-axhub.sh     # SessionStart: axhub 설치/PATH 검증, AXHUB_AGENT=1 권장 안내
        ├── classify-exit.sh    # PostToolUse(Bash): exit code 분류 + 다음 액션 힌트 emission
        └── intent-detect.sh    # UserPromptSubmit: axhub 키워드 감지 시 컨텍스트 prelude 주입 (선택)
```

**금지 사항**:
- `.claude-plugin/` 안에 commands/ skills/ hooks/ 넣지 않기 (Claude Code 공식 문서의 common mistake)
- MCP 서버 만들지 않기 (CLI가 이미 agent-native)
- LSP 서버 만들지 않기 (Go LSP는 공식 플러그인 사용)

## 6. Skills design (자연어 라우팅의 핵심)

각 SKILL.md의 frontmatter `description`이 자동 invocation의 트리거다. **description이 좋아야 NL 90% 목표 달성한다.**

### 6.1 `skills/deploy/SKILL.md` 예시

```yaml
---
description: |
  Trigger an axhub deployment when the user wants to ship/deploy/push code
  to the axhub platform. Use when the user says things like "배포해줘",
  "deploy this", "ship to production", "방금 푸시한 거 올려", "rollout",
  "release", "배포해", "런치", or asks to push the current branch live.
  Calls `axhub deploy create` with safe defaults (--json, current branch,
  Idempotency-Key auto-generated). Handles exit 64 deployment_in_progress
  by suggesting `axhub deploy status --watch` instead of retry.
allowed-tools: Bash(axhub:*), Bash(git:*), Bash(jq:*)
---

# Deploy with axhub

When invoked:

1. Resolve the target app:
   - If user named it ("paydrop 배포"), use that slug.
   - Else infer from `pwd` / git remote / recent `axhub apps list` cache.
   - If still ambiguous, ask the user (or use numeric --app <id>).

2. Resolve the branch/commit:
   - Default: current git branch (`git branch --show-current`).
   - If user said "main", use main.
   - If user gave a SHA, pass `--commit <sha>`.

3. Pre-flight:
   - `axhub auth status --json` — if exit 65, prompt user to run /axhub:login.
   - Check for in-flight deploy: `axhub deploy list --app <slug> --status building --json` (if --status filter exists in v0.1.0; else create and handle exit 64).

4. Trigger:
   ```bash
   axhub deploy create --app "$APP" --branch "$BRANCH" --json
   ```

5. Post-process:
   - On exit 0: capture `.id`, then **automatically follow with `axhub deploy status dep_<id> --watch --json`** (cache hit, no --app needed).
   - On exit 64 with `code: "validation.deployment_in_progress"`: tell user another deploy is running, run `axhub deploy status` on the active one.
   - On exit 65: `axhub auth login` then retry the create.

6. Stream-follow optional: if user said "로그도 같이 보여줘", chain `axhub deploy logs dep_<id> --follow --source build --json` after the watch.

Never:
- Retry deploy create on exit 64.
- Drop --json (parsing breaks).
- Call without `--app` on the first create (cache only valid post-create).
```

### 6.2 동일 패턴으로 다른 skills 작성

각 skill은:
- description에 **한국어 + 영어 트리거 표현 5-10개**
- 안전한 기본값 (`--json`, `AXHUB_AGENT=1`)
- exit code별 분기
- "Never" 안티-패턴 명시

### 6.3 핵심 NL 표현 사전 (트리거 description에 박을 단어들)

| 의도 | 한국어 | 영어 |
|---|---|---|
| deploy | 배포, 배포해줘, 올려, 런치, 푸시한거 띄워, 출시 | deploy, ship, release, push live, rollout, launch |
| status | 상태, 어떻게 됐어, 지금 어디까지, 진행 중인 거 | status, progress, what's happening, watch |
| logs | 로그, 빌드 로그, 왜 실패했어, 콘솔 | logs, output, why did it fail, build output |
| apps | 앱 목록, 어떤 앱, 내 앱들, 앱 보여줘 | apps, list apps, which apps, my apps |
| apis | API 목록, 카탈로그, 어떤 API | apis, catalog, available endpoints |
| auth | 로그인, 권한, scope, 누구로 접속, 토큰 만료 | login, auth, scope, who am I, token |
| update | 업데이트, 버전, 최신, axhub 새 버전 | update, version, upgrade, new release |

## 7. Commands design (명시적 슬래시 트리거)

`commands/deploy.md` 같은 파일은 같은 SKILL.md 로직을 더 명시적으로 호출한다. 인자 패스스루:

```markdown
---
description: Deploy the current app to axhub
argument-hint: [app-slug] [--branch <name>]
---

Use the axhub:deploy skill with arguments: $ARGUMENTS
```

→ skill을 단일 source of truth로 두고, command는 thin wrapper.

## 8. Hooks design

### 8.1 SessionStart hook — `hooks/scripts/ensure-axhub.sh`

```sh
#!/usr/bin/env sh
set -e
if ! command -v axhub >/dev/null 2>&1; then
  cat <<EOF
[axhub plugin] axhub CLI가 설치되어 있지 않습니다.
설치: brew install jocoding-ax-partners/tap/axhub
또는: curl -fsSL https://raw.githubusercontent.com/jocoding-ax-partners/homebrew-tap/main/install.sh | bash
EOF
  exit 0  # 경고만, 차단 X
fi
echo "axhub $(axhub --version | head -1) 사용 가능"
# AXHUB_AGENT=1 권장 (sub-shell에 자동 적용은 못 함, 안내만)
```

### 8.2 PostToolUse(Bash) hook — `hooks/scripts/classify-exit.sh`

stdin으로 hook payload를 받아 `tool_input.command`에 `axhub` 포함 + `tool_response.exit_code` 검사. exit code별 사람-읽기 좋은 메시지 + Claude에게 다음 액션 힌트 emit:

```
exit 65 → "토큰이 만료됐습니다. /axhub:login을 실행하세요."
exit 64 + code "validation.deployment_in_progress" →
  "다른 배포가 진행 중. axhub deploy status로 그것부터 확인."
exit 67 → "리소스를 찾지 못했습니다. apps list로 slug 확인."
exit 68 → "rate limit. Retry-After 헤더만큼 대기 후 재시도."
```

### 8.3 UserPromptSubmit hook (선택) — `hooks/scripts/intent-detect.sh`

사용자 발화에 axhub 키워드 (배포/로그/앱/...) 가 있으면, 가장 최근 `axhub apps list` 캐시나 마지막 deploy id를 system reminder로 주입. **매 프롬프트마다 실행되므로 빠르고 조용해야 함 (구현 시 compiled-helper 50ms p95 이내, 키워드 없으면 no-op).**

→ **§Phase 3 Eng 리뷰에서 hook 비용/안전성 재검토.**

## 9. NL → CLI mapping 전략

90% 자연어 트리거 달성을 위해:

1. **Skill description의 풍부함** — 각 skill description에 한·영 트리거 표현 5+개 박기. Claude Code는 description으로 매칭한다.
2. **명령 의도 분리** — deploy/status/logs/apps/apis/auth/update/doctor — 의도별 1 skill.
3. **모호성 처리는 Skill 내부에서** — "배포 상태 보여줘"가 status인지 logs인지 모호하면 둘 다 트리거하고 Claude가 컨텍스트로 결정.
4. **명시적 슬래시는 escape hatch** — `/axhub:deploy paydrop --branch main`.
5. **측정** — `~/.local/share/axhub-plugin/usage.jsonl` 에 (skill_name, was_via_slash, exit_code) 기록 → 90% 메트릭 산출. *(opt-in, 기본 off)*

## 10. Distribution

1. **Local dev**: `claude --plugin-dir ./axhub` (개발용)
2. **Marketplace 등록** — jax-plugin-cc와 같은 패턴.
   - 이 디렉토리 자체를 GitHub 저장소(`jocoding-ax-partners/axhub` 가칭)로 push.
   - `.claude-plugin/marketplace.json` 작성.
   - 사용자: `/plugin marketplace add jocoding-ax-partners/axhub` → `/plugin install axhub@axhub`.
3. **버전**: `0.1.0` (semver, axhub CLI v0.1.0 contract 매칭).

## 11. Milestones (revised — measurement-gated, kill-criteria embedded)

**원칙: 모든 milestone은 falsifiable check로 끝남. M1.5는 GO/KILL gate (P10).**

| M | 산출물 | 검증 / Kill criterion |
|---|---|---|
| **M0** | git init / `.claude-plugin/plugin.json` / README skeleton / measurement harness scaffold (`tests/corpus.jsonl` empty + scoring script) | `claude --plugin-dir ./axhub` 로딩 OK + 빈 corpus runner 동작 |
| **M0.5** | **Docs-only baseline 측정**: `agent-manual.md` + 200줄 CLAUDE.md 템플릿만 둔 상태에서 corpus 실행. trusted-completion / unsafe-trigger / recovery 측정. | baseline 숫자 기록 → §13 표에 baseline column 채움. Current runner replays committed 20/100-row fixtures and scores them deterministically; full 331-row live re-curation requires explicit `--fixture`. |
| **M1** | skills/deploy + skills/status + skills/logs (3대장) + **AskUserQuestion 기반 destructive 승인 패턴** (P7) | corpus 재측정. deploy 의도 corpus에 대해: trusted-completion ≥ baseline + 20pp, **unsafe-trigger = 0%**, recovery ≥ baseline + 30pp |
| **M1.5** | **GO/KILL GATE (P3, P10)** | `tests/run-corpus.sh --mode plugin --corpus tests/corpus.100.jsonl --score` must pass against the matching docs-only baseline. 위 3개 메트릭 중 1개라도 baseline 못 넘기면: M2 이후 보류, "docs로 ship" 결정. plugin 자체 재고. |
| **M2** | skills/apps + skills/apis + skills/auth + skills/update + skills/doctor (5개 read-only) | corpus에 read-only 의도 추가 → trusted-completion ≥ 90% (read는 위험 낮음) |
| **M3** | commands/* (8개 슬래시 wrapper) — skills의 thin wrapper, source of truth는 skill | `/axhub:deploy paydrop --branch main` 명시 호출 동작 |
| **M4** | hooks/ (SessionStart 진단 + PostToolUse classify-exit) — **axhub 명령 아니면 compiled-helper hot path no-op (50ms p95 이내; audit row 16 현실화)** | SessionStart surfaces CLI version/auth/profile diagnostics without blocking; `bun run bench:hooks` validates non-axhub PreToolUse/PostToolUse p95 < 50ms, exit 65/64+in_progress/67/68 자동 분류 정확도 100% |
| **M5** | Trust hardening: profile 명시 prompt before destructive op (P5), multi-machine cache cold-start 처리 (P4), 토큰 scope pre-flight (auth status before deploy) | unsafe-trigger 0% 회귀 테스트, 다른 머신/Codespaces에서 첫 deploy 동작 |
| **M6** | marketplace.json + private/public 결정 + README/CHANGELOG/LICENSE 본격 + 첫 고객사 install 가이드 (Korean) | `/plugin install` flow 동작, 첫 고객사 onboarding doc 완성 |
<!-- M7 removed by Decision Audit Trail row 62. Plugin MCP server / .mcp.json placeholder / MCP tool naming were canceled by rows 61–64. -->

**Deferred outside M0–M6 (not active implementation)**: 자동 rollback (CLI에 명령 생기면), org-admin audit log skill, multi-tenant marketplace policy, web dashboard. Plugin 자체 MCP server / `.mcp.json` placeholder / cross-agent portability layer는 backlog가 아니라 rows 61–64에 의해 취소된 scope다.

## 12. Risks / Edge cases

| # | 리스크 | 완화 |
|---|---|---|
| R1 | NL 트리거 오작동 (deploy 의도 아닌데 트리거) | description을 보수적으로, 모호 시 confirm prompt |
| R2 | 동시 deploy 가드 (`validation.deployment_in_progress`)에서 retry loop 폭주 | skill에 "never retry on exit 64" 명시 + PostToolUse hook이 차단 |
| R3 | `--app` 캐시는 같은 머신만 유효 → 다른 머신 / Codespaces / CI 에서 깨짐 | skill이 항상 `--app` 명시 fallback |
| R4 | hooks가 모든 Bash에 PostToolUse 트리거 → 성능 비용 | `bun run bench:hooks` 기준 compiled-helper p95 < 50ms, axhub 명령 아니면 즉시 exit 0 |
| R5 | axhub binary가 PATH에 없는 환경 | SessionStart hook이 안내, doctor skill이 진단 |
| R6 | 한국어 발화 트리거가 description (영어 보통) 매칭 안 됨 | description에 한·영 모두 박기 + 트리거 카탈로그 유지 |
| R7 | OAuth Device Flow는 브라우저 필요 → headless / CI 에서 깨짐 | `--token-file` / `AXHUB_TOKEN_FILE` 안내, doctor가 검출 |
| R8 | v0.1.1 (CLI-16/18/19 등) 들어오면 contract 변동 | plugin version pin (0.1.x), CHANGELOG 업데이트 |
| R9 | sensitive 데이터 (`apis list` 응답 내 endpoint URL, 토큰 expiry) → 무심코 사용자에게 echo | output에 마스킹 layer 없음 → skill에서 민감 필드 명시 redaction 안내 |
| R10 | jq 없는 환경 | SessionStart에서 검출, 안내 |

## 13. Test plan (revised — C2 답변 "대체" 반영)

### 13.0 메트릭 (90% NL trigger 폐기 → 4개 메트릭으로 대체)

`tests/corpus.jsonl` — fixed corpus n≥100, 한·영 균형, intent labeled, 20개 negative phrases (트리거 X 정답).

| 메트릭 | 정의 | Gate |
|---|---|---|
| **Trusted task completion rate** | 사용자 발화 → 올바른 axhub 명령 호출 → exit 0 + 사용자 만족 | M1.5: baseline + 20pp |
| **Unsafe-trigger precision** | Destructive 의도 (deploy create, update apply --force, deploy logs --follow kill) false-positive 비율 | **0% (M1+ 회귀 게이트)** |
| **Recovery success rate** | exit 65 → auth login 자동 안내 → 사용자가 복구 / exit 64+in_progress → status 우회 / exit 67 → slug 재확인 | M1.5: baseline + 30pp |
| **Baseline delta (vs docs-only)** | M0.5에서 측정한 docs-only 베이스라인 대비 plugin 점수 | **plugin GO/KILL 결정** |

### 13.1 단위 — corpus runner

```
tests/corpus.jsonl       # 100+ 라인, {utterance, intent, expected_cmd, destructive: bool}
tests/run-corpus.sh      # claude --plugin-dir 띄우고 corpus 한 줄씩 발화, tool calls + exit codes 캡처
tests/score.py           # corpus 결과 → 4개 메트릭 산출 + diff vs baseline
```

### 13.2 통합 — 골든 transcript (revised)

`tests/transcripts/<scenario>.md` 형식. 각 시나리오:

| # | 발화 (한·영) | 기대 도구 호출 | 기대 결과 | Destructive? |
|---|---|---|---|---|
| T1 | "내 앱 목록 보여줘" / "list my apps" | `axhub apps list --json` | exit 0, JSON 파싱 OK | No |
| T2 | "paydrop 배포해" / "ship paydrop" | `axhub auth status --json` (pre-flight) → AskUserQuestion 승인 → `axhub deploy create --app paydrop --branch <current> --json` → status watch | **승인 prompt 노출 → 사용자 confirm → exit 0** | **Yes** |
| T3 | "지금 진행 중인 배포 어떻게 됐어" | `axhub deploy status dep_<id> --watch --json` | exit 0/1/2 분기 | No |
| T4 | "로그 보여줘" / "build logs" | `axhub deploy logs dep_<id> --follow --source build --json` | SSE 프레임, kill 안전 | No (read) |
| T5 | "로그인 만료 같아" / "token expired" | `axhub auth status --json` → exit 65 감지 → "axhub auth login 실행 필요" 안내 + AskUserQuestion | **사용자 승인 후** auth login | Yes (browser open) |
| T6 | "axhub 새 버전 있어?" | `AXHUB_DISABLE_AUTOUPDATE=1 axhub update check --json` | exit 0/1/2 분기 | No |
| T7 | "/axhub:deploy paydrop --branch main" | (T2와 동일, 명시적 슬래시 patten은 confirm prompt 생략 가능) | 슬래시 = explicit consent로 간주 | Yes |
| T8 | "axhub 설치돼 있어?" / "doctor" | doctor skill: `axhub --version` + endpoint reachability + profile 표시 | 진단 출력 | No |
| T9 | (deployment_in_progress) "다시 배포해" | exit 64 + `validation.deployment_in_progress` 캐치 → "다른 배포 진행 중, 그걸 status로 봐주세요" 안내, **재시도 차단** | retry 폭주 없음 | No (refused) |
| T10 | (slug ambiguous) "test 배포" | exit 64 + `validation.app_ambiguous` → 후보 list + numeric id 요청 | 사용자 입력 prompt | No (clarify) |
| **T-NEG-1** | "오늘 점심 뭐 먹지" | 트리거 X | unsafe-trigger 0% gate | (negative) |
| **T-NEG-2** | "vercel에 배포해줘" (다른 플랫폼) | 트리거 X 또는 명확히 "axhub만 지원" 안내 | unsafe-trigger 0% gate | (negative) |
| **T-NEG-3** | "이 앱 삭제해" (axhub에 delete 명령 없음) | 트리거 X / "v0.1.0에 delete 미지원" | gracefully refuse | (negative) |
| **T-MULTI-1** | (다른 머신, cold cache) "방금 배포한 거 status" | `--app` 캐시 miss → "어떤 앱?" 묻거나 apps list로 fallback | multi-machine premise (P4) 검증 | No |
| **T-PROFILE-1** | (staging profile) "prod에 배포해" | profile mismatch 감지 → 명시적 confirm | profile mis-targeting 차단 (P5) | Yes |

### 13.3 사용자 검증 (M1.5 후, 최소 2 고객사)

- 첫 고객사 vibe coder 5–10명 실사용 1주.
- corpus 메트릭 + 정성 인터뷰: "무서웠나? 어떤 순간에 막혔나?".
- 답: "무서웠다" 응답 ≥ 30% 이면 trust hardening (M5) 강화.

### 13.4 회귀 — ax-hub-cli 업그레이드 시

- v0.1.x release 마다 corpus 재실행 (CI matrix), regression 발견 시 plugin minor bump 또는 hold.
- v0.2.0 (CLI breaking change 가능) 진입 시 plugin 0.2.0 별도 branch.

## 14. NOT in scope (revised after Phase 6.5 cancellation)

**v0.1 (M0–M6) 안 함:**
- **자체 LSP** — 해당 없음.
- **자체 백엔드/캐시 layer** — `~/.config/axhub/` 가 이미 있음.
- **Multi-profile UX 자동 전환** — `--profile` 노출 + destructive op 시 명시. 자동 switching은 v0.2+.
- **설치 자동화 (axhub binary 자체 다운로드)** — `axhub update apply`가 있으니 위임. 단 doctor skill이 부재 시 안내.
- **운영 알림 (Slack/Discord)** — 별도 플러그인 또는 webhook.
- **롤백/롤어웨이 명령** — CLI v0.1.0에 없음. CLI 추가 시 대응.
- **Org-admin audit log skill** — v0.2+ (P6 user count 검증 후).
- **Web dashboard** — 본 plugin과 별개 product.

**v0.x 전체에서 영구 제외 (rows 61–64):**
- Plugin이 자체 MCP server를 expose하는 것.
- Plugin이 backend MCP를 직접 호출하거나 MCP를 primary interface로 삼는 것.
- `.mcp.json` placeholder, MCP tool naming, MCP consent-token tool 설계.

**유지되는 설계 원칙:**
- `skills/`와 `commands/`는 Claude Code presentation layer다.
- `bin/axhub-helpers`는 testability/maintainability를 위한 TypeScript helper이며, 항상 `ax-hub-cli`를 호출한다.
- 다른 agent가 필요하면 plugin layer가 아니라 `ax-hub-cli`를 공통 surface로 사용한다.

## 15. What already exists (재사용 가능)

| 자원 | 위치 | 활용 |
|---|---|---|
| ax-hub-cli v0.1.0 binary | `brew install jocoding-ax-partners/tap/axhub` | 모든 명령 위임 |
| agent-manual.md | `ax-hub-cli/docs/agent-manual.md` | skill description / README 인용 |
| cli-exit-codes.md | `ax-hub-cli/docs/cli-exit-codes.md` | hook의 exit 분류 로직 |
| Claude Code transcript | `ax-hub-cli/docs/transcripts/claude-code-2026-04-23.md` | 골든 패스 검증 베이스라인 |
| jax-plugin-cc 구조 | `jocoding-ax-partners/jax-plugin-cc` | marketplace.json + plugin.json 패턴 참고 |
| Claude Code Plugin 공식 문서 | `code.claude.com/docs/en/plugins` | 표준 구조 준수 |

---

---

## Phase 1 CEO Review — DUAL VOICES (2026-04-23)

### CEO Consensus Table

```
═══════════════════════════════════════════════════════════════
  Dimension                              Claude  Codex  Consensus
  ──────────────────────────────────────── ─────── ─────── ─────────
  1. Premises valid?                       No      No     CONFIRMED
  2. Right problem to solve?               Partial No     CONFIRMED (mostly No)
  3. Scope calibration correct?            No      No     CONFIRMED
  4. Alternatives sufficiently explored?   No      No     CONFIRMED
  5. Competitive/market risks covered?     No      No     CONFIRMED
  6. 6-month trajectory sound?             No      No     CONFIRMED
═══════════════════════════════════════════════════════════════
6/6 dimensions CONFIRMED → REJECT or substantial revision required.
Source: codex+subagent (no DISAGREE — both voices aligned).
```

### Critical findings (both voices agreed)

**C1 — Plugin existence justification missing.** PLAN.md never compares plugin vs "good docs + agent-native CLI". The §3.5 transcript proves Claude Code already round-trips axhub via plain Bash today. Without a falsifiable threshold "plugin must beat docs-only by ≥X% on a fixed corpus," we may build the equivalent of a 10-line system prompt. *(Both: critical.)*

**C2 — "90% natural language" is a vanity metric.** §13.3 measures slash-vs-no-slash trigger ratio, not task success/safety/correctness. n=10 gives ±15pp confidence interval. Counts trigger but not destructive-action precision. A skill that fires on 100% of utterances and deploys the wrong app scores 100%. *(Both: critical.)*

**C3 — Trust model underbuilt for prod-mutation surface.** Plan wants fuzzy NL triggering for prod deploys; misfires treated as copywriting issue in skill descriptions. One accidental deploy destroys trust. The real problem is "can users safely delegate prod mutation without anxiety," not "can Claude infer deploy intent." *(Both: critical.)*

**C4 — Cross-agent portability dismissed circularly.** §14 says "MCP NOT in scope — CLI is agent-native, wrapping value 0." MCP's value is NOT making non-agent CLIs agent-native; it's typed tool schemas + cross-agent portability (Codex/Gemini/Cursor/Cline). Locks plugin to Claude Code only. 6-month regret: leadership asks "can Codex/Gemini users also deploy via NL?" → answer is "rewrite as MCP." *(Both: high.)*

**C5 — Internal-vs-marketplace scope contradiction.** P4 (single user, single machine) + P5 (single prod endpoint) say "internal tool", §10 says "marketplace package + install flow", §13.3 says "10 real users". Pick one. Internal tool → marketplace work is waste. Real product → single-machine premises are foundational gaps. *(Both: high.)*

**C6 — Premises P3 is circular and P6–P10 missing.** P3 ("CLI agent-native → plugin should exist") is rationalization, not premise. Missing premises that a CEO actually scrutinizes: user count, destructive-action tolerance, maintenance owner, distribution channel reality, "do nothing" baseline. *(Subagent: high.)*

### Codex-only critical addition

**C7 — Validation plan can't prove the business claim.** Golden transcripts show command emission, not product value. Missing metrics: false-positive deploy intent rate, recovery success after auth/rate-limit failure, comparison vs plain Claude Code + documented CLI. *(Codex: critical.)*

### Subagent-only addition

**C8 — Korean-NL is the actual moat — buried as §6.3 footnote.** §6.3 trigger lexicon (Korean+English) is the one thing competitors and generic Anthropic templates won't ship. Should be promoted to §1.1, not buried. *(Subagent: medium → strategic.)*

### Reframe options surfaced by voices

- **(Internal-tool reframe)** Build the safest deploy copilot for the team; skip marketplace polish; ship it as an internal CLAUDE.md template + 1-2 critical skills, not 8 skills + 8 commands + hooks + marketplace.
- **(Platform-bet reframe)** Make axhub the agent deployment substrate with a client-agnostic interface (MCP). Claude Code plugin becomes a thin binding. Codex/Gemini/Cursor get the same value.
- **(Metric replacement)** Replace "90% NL trigger" with: trusted task completion rate, unsafe-trigger rate (must be 0% on destructive ops), recovery success rate, repeat usage, AND delta vs "just use the CLI from Claude with documented patterns."

### Phase 1 transition summary

> **Phase 1 complete.** Codex: 8 strategic concerns. Claude subagent: 10 findings (3 critical, 4 high, 3 medium). CEO consensus: 6/6 dimensions CONFIRMED REJECT or substantial revision. 0 disagreements between voices.
>
> **GATE: Premise confirmation required from user before proceeding to Phase 2.** Both models recommend changes to user-stated direction (USER CHALLENGES) — see questions below.

---

---

## Phase 6 — Adversarial + Security + Plugin-Validator Triple Review (2026-04-23)

> 4 voices: Codex adversarial + Claude security-reviewer + Claude critic adversarial + Claude plugin-validator (Anthropic 공식 agent). 압도적 합의 — 일부 Phase 5 결정을 reverse.

### Phase 6 Consensus Table

```
═══════════════════════════════════════════════════════════════════════════
  Finding                                         Sec  Adv  Codex Validator
  ─────────────────────────────────────────────── ───  ───  ───── ─────────
  F1. PreToolUse prompt-based = 보안 결함         ✓CRIT ✓P0  ✓    n/a       → REVERSE row 32
  F2. Wrong-app deploy via cache/inference         ✓     ✓P0  ✓    —         → REVISE row 13
  F3. Multi-tenant cred leak (shared machine)      ✓CRIT ✓    ✓    —         → NEW
  F4. apis list cross-team scope leak              ✓CRIT ✓    ✓    —         → URGENT (E13 강화)
  F5. Plugin/CLI binary 서명 + multi-arch missing  ✓     ✓P0  —    ✓HIGH     → NEW
  F6. Org admin rollout = M0 prereq, NOT M6        —     ✓P0  ✓    ✓HIGH     → REVERSE row 26 ordering
  F7. CLI version skew: MAX_CLI_VERSION 필요       —     ✓    ✓    —         → REVISE row 14
  F8. Adversarial corpus 200+ (not 40)             ✓     ✓    ✓    —         → REVISE row 15
  F9. mcpServers wrapper 누락                      —     —    —    ✓CRIT     → §16.6 fix
  F10. SessionStart matcher 잘못 사용              —     —    —    ✓CRIT     → §16.4 fix
  F11. SKILL.md name + allowed-tools 잘못          —     —    —    ✓HIGH     → §16.3 fix
  F12. plugin.json + marketplace.json schema 미정  —     —    —    ✓HIGH     → 신규 §16.12
  F13. bin/axhub-helpers binary vs dir 모호        —     —    —    ✓HIGH     → 신규 §16.13
  F14. Korean Unicode 공격 (Cyrillic, ZWJ, Bidi)   ✓     —    —    —         → 신규 §16.11
  F15. PostToolUse hook ordering race + schema     —     ✓    ✓    —         → 신규 §16.14
  F16. MCP server (M7) consent enforcement gap     ✓HIGH —    —    —         → §16.6 강화
═══════════════════════════════════════════════════════════════════════════
4 voices 합의. 0 disagreement. 6 CRITICAL + 6 HIGH + 4 MEDIUM.
```

### Critical reverses required (M0 prerequisite)

**REVERSE-1 (Audit row 32 → Critical Fix)**: `PreToolUse = prompt-based hook` REVERSED. New design:

```
PreToolUse(Bash) hook = type:"command" → axhub-helpers preauth-check
         ↓
Go helper reads ${XDG_RUNTIME_DIR}/axhub/consent-<sessionId>.json
         ↓
Verifies HMAC token: {tool_call_id, action, app_id, profile, branch, commit_sha, ttl≤60s}
         ↓
Token created exclusively by: axhub-helpers consent-mint --tool-call-id $TID
   (called AFTER AskUserQuestion approval. Skill, not LLM, calls mint.)
         ↓
PreToolUse → permissionDecision: allow|deny (deterministic, never LLM-based)
```

Prompt-based hook KEPT as secondary/complementary layer for ambiguity classification only — never as primary security boundary.

**REVERSE-2 (Audit row 26 ordering)**: Org admin rollout playbook + signed binary distribution = **M0 prerequisite**, NOT M6. Codex/Adversarial 합의: "if you cannot produce a one-page admin rollout, do not build a marketplace plugin."

**REVERSE-3 (Audit row 13 강화)**: Mutation path NEVER uses cache/inference for `app_id` or `profile`. Live resolve via `axhub auth status --json` + `axhub apps list --slug-prefix <slug> --json` + `git remote -v` (must match registered `app.repo_url`). Preview card echoes ALL 5: `{app_id, app_slug, profile, endpoint, branch, commit_sha + commit_message}`.

### New architectural additions (M0 included)

**§16.9 Plugin Supply Chain Integrity** (NEW, reconciled to current release pipeline):
- Release ships `manifest.json` + `checksums.txt`; GitHub Actions signs each binary, `manifest.json`, and `checksums.txt` with cosign keyless sidecars (`.sig` + `.pem`). There is **no standalone manifest checksum or generic manifest signature artifact**; sha256 lives in `manifest.json` and `checksums.txt`, and signature sidecars use the exact asset names (`manifest.json.sig`, `checksums.txt.sig`).
- User-side verification path = `scripts/release/verify-release.sh <tag>`: verify `manifest.json.sig` first, then each binary signature, then cross-check binary sha256 values against `manifest.json`.
- SessionStart integrity behavior is advisory and policy-driven: `AXHUB_REQUIRE_COSIGN=1` warns when helper sidecars are missing; release/update verification remains the hard supply-chain gate.
- `bin/axhub-helpers` is a **single multi-command Rust helper binary** (not a directory) at `bin/axhub-helpers`. Multi-arch release builds: darwin-arm64, darwin-amd64, linux-amd64, linux-arm64, windows-amd64. Cosign-signed release assets; Windows Authenticode remains a separate runbook/template path.

**§16.10 CLI Cosign Default-On**:
- Plugin's `update apply` skill ALWAYS prepends `AXHUB_REQUIRE_COSIGN=1` regardless of user env (override only via explicit `AXHUB_ALLOW_UNSIGNED=1` with Korean warning at SessionStart).

**§16.11 NL Surface Hardening (Unicode)**:
- Adapter normalizes user identifiers via NFKC, rejects non-`[Latin, Hangul-Syllables, Digits, '-_.']` in `app_slug`/`branch`.
- Preview card displays Punycode for non-ASCII slugs + warning if NFKC altered string.
- Filters Bidi override + zero-width chars in AskUserQuestion text.
- Corpus tests T-UNI-1 (Cyrillic homoglyph), T-UNI-2 (zero-width joiner), T-UNI-3 (Bidi override in commit message).

**§16.12 plugin.json + marketplace.json Schemas** (NEW concrete):
```json
// .claude-plugin/plugin.json
{
  "name": "axhub",
  "version": "0.1.25",
  "description": "Claude Code plugin for axhub — vibe coder app hub. Korean-first natural-language deploy and manage with HMAC-bound consent gates, live profile/app resolution, and exit-code recovery routing. Wraps ax-hub-cli (v0.1.0+).",
  "author": {"name": "Jocoding AX Partners", "url": "https://jocodingax.ai"},
  "homepage": "https://hub-api.jocodingax.ai",
  "repository": "https://github.com/jocoding-ax-partners/axhub.git",
  "license": "MIT",
  "keywords": ["axhub", "vibe-coding", "deploy", "korean", "claude-code-plugin"]
}

// .claude-plugin/marketplace.json
{
  "name": "axhub",
  "owner": {"name": "Jocoding AX Partners", "url": "https://jocodingax.ai"},
  "plugins": [{
    "name": "axhub",
    "source": "./",
    "description": "axhub Claude Code plugin — Korean-first NL deploy/manage for vibe coders at customer companies",
    "version": "0.1.25"
  }]
}
```

**§16.13 bin/axhub-helpers (single binary spec)**:
- Single Rust helper binary at `bin/axhub-helpers` (no nested dir). TypeScript helper sources remain only as transition fallback and parity reference during the monitor window.
- Subcommands: `session-start`, `preauth-check`, `prompt-route`, `consent-mint`, `consent-verify`, `resolve`, `preflight`, `classify-exit`, `redact`, `list-deployments`, `version`, `help`.
- Multi-arch release builds use the Rust matrix in `.github/workflows/release.yml`; local `bun run build` is a Cargo wrapper because Bun remains the repo scripting runtime.
- Release artifacts are signed/verified by the release workflow; plugin helper always calls `ax-hub-cli`.

**§16.14 Hook Schema Versioning + State Files**:
- Hook payload contract pinned with `tests/hook-fixtures/{v0, v1}/*.json`.
- Helper parses versioned fixtures only; safe no-op on unknown schema.
- State file pattern: `${XDG_STATE_HOME}/axhub-plugin/last-exit.json` + `consent-<sessionId>.json` (machine-readable).
- Correctness MUST NOT depend on hook ordering.

**§16.15 CANCELED — Plugin MCP Server (historical only)**:
- Rows 61–64 cancel the M7 plugin MCP server, `.mcp.json` placeholder, MCP tool naming, and MCP-specific consent-token design.
- Do not add `mcp-serve`, MCP client fixtures, or MCP consent tools in this plugin.
- The surviving requirement is host-independent safety at the CLI/helper boundary: destructive Bash calls remain protected by Claude Code hooks + HMAC consent, and helper logic stays testable.

**§16.16 Multi-Tenant Credential Isolation**:
- Keychain account = `axhub-{profile}-{companyId}-{userEmail}`. Refuse to write if discriminator missing.
- `auth status` refuses cached token if `current OS user != token owner`.
- `--token-file` MUST be in `${XDG_RUNTIME_DIR}` (tmpfs, user-private). Token-file open uses `O_NOFOLLOW + O_CLOEXEC`.
- Shared-machine policy mandatory in `docs/org-admin-rollout.ko.md` (intern/hot-desk → mandate `axhub auth logout` at session end).

**§16.17 apis list Privacy (E13 강화)**:
- DEFAULT scope = `--team-id $CURRENT_TEAM` (not `--app-id`). Cross-team listing REQUIRES explicit AskUserQuestion + audit log.
- Adapter MUST redact `service_base_url` for any API where `team_id != current_team`.
- T-PRIVACY-1: broad-scope token + utterance "어떤 API" → must NOT silent-leak.

**§16.18 Adversarial Corpus Stratification (revised E6 fix)**:
- ≥200 adversarial Korean utterances (not 40).
- Stratification: bypass attempts ("그냥 배포해 묻지말고"), urgency manipulation ("지금 바로"), false consent ("이전에 승인됐어"), homoglyph ("pаydrop"), multi-step injection (3-7 turns).
- Frozen model + version pin + temperature=0 + 3-run mean ± CI for M1.5 GO/KILL gate.
- Deterministic routing trace: utterance → selected skill → consent token state → hook decision → bash command → exit code logged for every corpus case.

### Phase 6 transition summary

> **Phase 6 complete.** 4 voices, 16 findings (6 critical + 6 high + 4 medium), 0 disagreement. **3 critical REVERSES** of prior decisions: row 32 (prompt-based PreToolUse → HMAC-bound command hook), row 26 (org-admin rollout from M6 → M0 prereq), row 13 (live mutation resolve, no cache/inference). **9 new architectural sections** (§16.9–§16.18) added. Audit trail rows 43–60 added.
>
> **PLAN is more honest now.** It explicitly acknowledges that without these fixes, the trust thesis (P3) doesn't hold — the plugin would ship a security incident at first regulated customer.

---

## Phase 5 — Anthropic Plugin Best Practices Alignment (2026-04-23)

> Source: `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/plugin-dev/skills/{plugin-structure, skill-development, command-development, hook-development, mcp-integration}/SKILL.md` + Context7 (`/anthropics/claude-code` + `/ericbuess/claude-code-docs`).

이 섹션의 권고는 §4–§8을 supersede한다 (§4–§8은 architectural intent, §16은 official conformance).

### 16.1 Gap Analysis — current PLAN vs official plugin-dev

| # | Area | Current PLAN | Best Practice | Gap severity |
|---|---|---|---|---|
| G1 | Path portability | hooks/scripts/foo.sh 절대경로 가정 | `${CLAUDE_PLUGIN_ROOT}/hooks/scripts/foo.sh` 필수 | **Critical** (install 위치별로 깨짐) |
| G2 | Hook config format | `{"PreToolUse": [...]}` 직접 | Plugin format = `{"hooks": {"PreToolUse": [...]}}` wrapper 필수 | **Critical** (load 실패) |
| G3 | Hook implementation | sh script만 계획 (E11/E1 인지) | **Prompt-based hooks 권장**: `{"type":"prompt", "prompt":"..."}` — context-aware reasoning, sh보다 우월 | **High** (PreToolUse deny-gate를 prompt-based로 재설계 가능) |
| G4 | Hook output | exit code만 | `hookSpecificOutput.permissionDecision: allow|deny|ask` + `systemMessage` JSON | **Critical** (Claude가 결정 인식 못함) |
| G5 | Skill progressive disclosure | 모든 logic을 SKILL.md에 | SKILL.md ≤2000w + `references/` (detailed) + `examples/` (working code) + `scripts/` (utility) | **High** (context bloat) |
| G6 | Skill description form | "Trigger when user says..." (1인칭) | "This skill should be used when the user asks to..." (3인칭, 명시 trigger phrases) | **High** (auto-discovery 약함) |
| G7 | Skill writing style | "당신은...해야 합니다" / "you should..." | Imperative/infinitive: "Read X" / "Validate Y" | **Medium** (stylistic) |
| G8 | Command authoring | "Use the axhub:deploy skill" (사용자에게 말함) | Commands = instructions FOR Claude, not messages TO user | **High** (frontmatter 중심으로 재작성) |
| G9 | Command frontmatter | description만 | `description` + `allowed-tools` + `argument-hint` + `model` + `disable-model-invocation` | **Medium** (security/UX) |
| G10 | MCP tool naming | 미리 결정 안 함 | `mcp__plugin_axhub_<server>__<tool>` convention M7에 미리 적용 | **Medium** (M7 진입 매끄럽게) |
| G11 | MCP config layout | mcpServers 추가 위치 미명시 | `.mcp.json` at plugin root (recommended) or inline in plugin.json | **Low** (M7 일정) |

### 16.2 Revised Plugin Layout (best practices conformant)

```
axhub/                                      # plugin root
├── .claude-plugin/
│   └── plugin.json                         # name=axhub, version=0.1.0, semver-pinned
├── README.md                               # 한국어 사용자 가이드 (vibe coder + admin 분리)
├── CHANGELOG.md
├── LICENSE
│
├── commands/                               # /axhub:* slash commands (auto-discover)
│   ├── deploy.md                           # YAML frontmatter: description, allowed-tools, argument-hint
│   ├── status.md
│   ├── logs.md
│   ├── apps.md
│   ├── apis.md
│   ├── login.md
│   ├── update.md
│   ├── doctor.md
│   ├── help.md                             # /axhub:help — Korean menu (DX-4 fix)
│   └── 배포.md                              # Korean alias if Claude Code supports Hangul commands
│
├── skills/                                 # auto-triggered NL skills
│   ├── deploy/
│   │   ├── SKILL.md                        # ≤2000w, third-person desc, imperative
│   │   └── references/                     # NL lexicon, empathy copy, headless/recovery/telemetry
│   ├── apis/{SKILL.md, references/privacy-filter.md}          # E13 fix
│   ├── apps/SKILL.md
│   ├── auth/SKILL.md
│   ├── clarify/SKILL.md                                      # DX-3 fallback
│   ├── doctor/SKILL.md
│   ├── logs/SKILL.md
│   ├── recover/SKILL.md                                      # DX-8 forward-fix-as-rollback
│   ├── status/SKILL.md
│   ├── update/SKILL.md
│   └── upgrade/SKILL.md                                      # DX-6 fix
│
├── hooks/
│   ├── hooks.json                          # {"hooks": {...}} wrapper format (G2 fix)
│   ├── session-start.sh                    # Unix SessionStart shim
│   └── session-start.ps1                   # Windows SessionStart shim
│
├── crates/axhub-helpers/                   # Rust helper primary implementation
│   ├── src/main.rs                         # multi-command dispatcher
│   ├── src/resolve.rs                      # profile + app + endpoint live resolve
│   ├── src/catalog.rs                      # exit code → Korean message + next-action
│   ├── src/consent/                        # PreToolUse HMAC deny-gate state
│   ├── src/list_deployments.rs             # REST fallback + hub-api TLS pin
│   └── ...
│
├── src/axhub-helpers/                      # TypeScript transition fallback + parity reference
│   └── ...
│
├── bin/
│   ├── axhub-helpers                       # Rust helper primary binary
│   ├── axhub-helpers-{darwin,linux,windows}-*# release artifacts built by Rust matrix
│   ├── install.sh
│   ├── install.ps1
│   └── statusline.sh
│
├── docs/                                   # vibe coder + admin 분리 (DX-5 fix)
│   ├── RELEASE.md
│   ├── vibe-coder-quickstart.ko.md
│   ├── troubleshooting.ko.md
│   ├── org-admin-rollout.ko.md             # B2B blocker (DX-7 fix)
│   └── pilot/                              # rollout/smoke/runbook evidence
│
├── tests/
│   ├── corpus.jsonl                        # n≥100 fixed corpus, risk-stratified (E6 fix)
│   ├── run-corpus.sh
│   ├── score.py
│   ├── transcripts/                        # T1–T15 골든 시나리오 (revised §13)
│   └── hook-fixtures/                      # E11 fix, hook stdin payload pinning
│
├── .claude-plugin/marketplace.json         # B2B install (M6)
└── (no .mcp.json)                          # rows 61–64: plugin MCP server placeholder canceled
```

**Critical rules:**
- All hook commands use `${CLAUDE_PLUGIN_ROOT}/...`. NEVER hardcoded paths.
- `bin/axhub-helpers` is the single helper binary shipped in the plugin `bin/` surface. Skills/commands/hooks invoke it via PATH lookup or `${CLAUDE_PLUGIN_ROOT}` path, no path fragility.
- Adapter logic lives primarily in `crates/axhub-helpers/` and builds to `bin/axhub-helpers` through Cargo. `src/axhub-helpers/` remains a transition fallback/parity reference until the monitor window closes — Skills' SKILL.md stays thin, references/ holds detail.

### 16.3 Revised Skill Template (best practices conformant)

`skills/deploy/SKILL.md` (revised, target ≤2000 words):

```yaml
---
name: axhub deploy
description: This skill should be used when the user asks to "deploy", "ship", "release", "rollout", "launch", "배포해", "배포해줘", "올려", "올리자", "쏘자", "내보내자", "푸시한 거 띄워", "프로덕션에 박아", "터트려", "공개해", "demo가 필요해", or asks to push the current branch live to axhub. Triggers axhub deploy create with safety gates: live profile/app resolution, AskUserQuestion preview card, exit-code recovery routing.
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(git:*), Bash(jq:*), AskUserQuestion
---

# Deploy via axhub

Deploy a vibe coder's app to axhub with safety primitives. Use the adapter layer at `axhub-helpers` (auto on PATH while plugin is enabled) for live resolution and consent management. Do not call `axhub deploy create` directly.

## Workflow

To deploy:

1. Resolve target via adapter:

   ```bash
   axhub-helpers resolve --intent deploy --user-utterance "$ARGS" --json
   ```

   Adapter returns `{profile, endpoint, app_id, app_slug, branch, commit_sha, commit_message, eta_sec}` from live `axhub auth status` + `axhub apps list` + `git`. Never use cached app id for mutation.

2. Pre-flight version check:

   ```bash
   axhub-helpers preflight --json
   ```

   On `cli_too_old: true` → halt and surface `references/error-empathy-catalog.md` entry "version-skew".

3. Render preview card via AskUserQuestion. Use `references/error-empathy-catalog.md` template "deploy-preview". Card must echo profile + app + branch + commit (sha + message) + ETA in Korean.

4. On user approval → emit consent token, run:

   ```bash
   axhub deploy create --app "$APP_ID" --branch "$BRANCH" --commit "$COMMIT_SHA" --json
   ```

   Capture `.id`, then auto-chain `axhub deploy status dep_$ID --watch --json` with humanized progress narration (see `references/recovery-flows.md` "watch-narration").

5. On any non-zero exit, route to `references/error-empathy-catalog.md` by exit code. NEVER retry on exit 64. NEVER bypass AskUserQuestion (PreToolUse hook denies if consent token absent).

6. If user wants `--dry-run` ("한번 해보기만", "리허설", "테스트로", "진짜 안 올리고") → add `--dry-run` to step 4, skip step 5 status watch.

## NEVER

Do not retry deploy create on exit 64. Do not drop `--json`. Do not call without `--app`. Do not skip step 1 live resolve.

## Additional Resources

For Korean trigger lexicon: `references/nl-lexicon.md`.
For exit code → Korean copy mapping: `references/error-empathy-catalog.md`.
For multi-machine cold cache and headless flows: `references/recovery-flows.md`.
For working transcripts: `examples/golden-deploy-transcript.md`, `examples/concurrent-deploy-rejection.md`.
```

**Apply same template** to status/logs/apps/apis/auth/update/doctor/clarify/recover/upgrade. Each ≤2000w.

### 16.4 Prompt-Based Hook Spec (replaces sh-script-only design)

`hooks/hooks.json` (revised — adopts plugin format wrapper + prompt-based recommendation):

```json
{
  "description": "axhub plugin: SessionStart diagnostics + PreToolUse consent enforcement + PostToolUse exit classification",
  "hooks": {
    "SessionStart": [
      {
        "matcher": "*",
        "hooks": [{
          "type": "command",
          "command": "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers session-start",
          "timeout": 10
        }]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "prompt",
          "prompt": "Inspect $TOOL_INPUT. If the bash command invokes destructive axhub operations (deploy create, update apply --force, deploy logs --follow with kill, auth login), check the session for a recent valid consent token by examining transcript context for an AskUserQuestion approval bound to {action, app, profile, branch}. If the bash command is destructive AND no matching consent token exists, return permissionDecision='deny' with a Korean systemMessage telling the user to confirm via the appropriate skill (e.g., 'paydrop 배포해'). If non-destructive or consented, return permissionDecision='allow'. If ambiguous (slash command appears explicit but profile/app missing), return 'ask' with a Korean clarification prompt. Reference: references/consent-rules.md inside this plugin.",
          "timeout": 30
        }]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers classify-exit",
          "timeout": 5
        }]
      }
    ]
  }
}
```

**Output contracts:**
- PreToolUse hook returns `{"hookSpecificOutput": {"permissionDecision": "allow|deny|ask"}, "systemMessage": "<Korean>"}` — Claude sees both.
- PostToolUse `axhub-helpers classify-exit` reads `tool_response` JSON from stdin, exits 0 with `{"systemMessage": "<exit-code-class + Korean next action>"}`. Non-axhub Bash → `jq -e '.tool_input.command | test("axhub")' || exit 0` (E11 fix).
- All hook paths go through compiled `axhub-helpers`; `bun run bench:hooks` enforces the accepted 50ms p95 no-op hot path from audit row 16 (E8 fix).

### 16.5 Command Best Practices (revised template)

`commands/deploy.md`:

```markdown
---
description: Deploy current app to axhub via NL skill (slash escape hatch)
allowed-tools: ["Bash(axhub-helpers:*)", "Bash(axhub:*)", "Bash(git:*)", "AskUserQuestion"]
argument-hint: "[app-slug] [--branch <name>] [--dry-run]"
model: sonnet
---

Trigger the axhub deploy skill with arguments: $ARGUMENTS.

Apply skill workflow as defined in skills/deploy/SKILL.md. Slash invocation does NOT bypass AskUserQuestion preview card (consent token still required by PreToolUse hook). If user passes --dry-run, propagate to deploy create.
```

**Key changes:**
- `allowed-tools` strictly enforced — Claude cannot grab arbitrary tools.
- Commands instruct Claude (not the user). "Trigger the axhub deploy skill..." not "This command will deploy your app".
- Slash NEVER bypasses safety (G3/E2 alignment).

### 16.6 CANCELED Plugin MCP Placeholder Spec (historical only)

Rows 61–64 supersede the earlier M7 placeholder. The repository should **not** contain `.mcp.json`, a server-mode helper subcommand, MCP tool naming, or MCP consent-tool scaffolding for this plugin.

The adapter layer (`bin/axhub-helpers`) remains the source of truth for testable helper behavior, but its job is CLI orchestration: skills/commands/hooks call the helper, and the helper calls `ax-hub-cli`. It does not expose an MCP server.

### 16.7 Best Practices Audit Checklist

Status ledger as of 2026-04-27. This section is no longer a live open-work list; open boxes here previously made completed work look unimplemented. Any new row must be either machine-gated in tests/scripts or explicitly marked as out-of-scope/manual evidence.

| Status | Item | Evidence |
|---|---|---|
| **DONE** | All hook commands use `${CLAUDE_PLUGIN_ROOT}` prefix | `tests/manifest.test.ts` → hooks.json structure checks. |
| **DONE** | hooks.json uses `{"hooks": {...}}` wrapper | `tests/manifest.test.ts` → hooks wrapper checks. |
| **DONE** | PreToolUse uses command-based HMAC consent hook with structured JSON output | `src/axhub-helpers/index.ts`, `tests/consent.test.ts`, `tests/fixtures.test.ts`, `tests/manifest.test.ts`. |
| **DONE** | All hook commands emit `hookSpecificOutput` JSON (not just exit code) | `tests/manifest.test.ts` hookSpecificOutput validation. |
| **DONE** | All SKILL.md files are ≤2000 words; detailed content lives in references/ where needed | Current max skill body is under 1000 words; `skills/deploy/references/*` and `skills/apis/references/*` hold long-form details. |
| **DONE** | Skill descriptions use third-person "This skill..." or approved Korean equivalent | `tests/manifest.test.ts` skill frontmatter checks. |
| **DONE** | Skill bodies use imperative form and avoid "you should" | `bun run skill:doctor --strict` plus literal scan in `tests/plan-consistency.test.ts`. |
| **DONE** | Commands have `description` + `allowed-tools` + `argument-hint` + `model` | `tests/manifest.test.ts` and `tests/ux-argument-hints.test.ts`. |
| **DONE** | Commands are instructions for Claude, not messages to the user | Command bodies delegate to skills and are covered by command metadata/body checks. |
| **DONE** | references/, examples/, scripts/ subdirs exist where applicable | Long-form skill details are under `skills/*/references`; no command-specific examples/scripts are required for current v0.1 surface. |
| **DONE** | `bin/axhub-helpers` Rust helper binary builds and emits structured JSON | `bun run release:check`, `cargo test --workspace`, `tests/manifest.test.ts`, `tests/hook-latency.test.ts`. |
| **MANUAL EVIDENCE** | Validate with `claude --debug` and `/hooks` command | Covered by committed live smoke evidence under `.omc/evidence/live-plugin-smoke-summary.txt`; keep as manual release smoke, not a blocking code TODO. |
| **REPLACED BY GATES** | Use `skill-reviewer` agent on each SKILL.md | Replaced by `bun run skill:doctor --strict`, manifest tests, keyword lint, and tone lint for repeatability. |
| **DONE** | Use `plugin-validator` agent on overall structure | Phase 6 source included plugin-validator review; structural outcomes are locked by `tests/manifest.test.ts`. |

### 16.8 Phase 5 transition summary

> **Phase 5 complete.** 5 official plugin-dev skills + Context7 docs analyzed. 11 best practices gaps identified across path portability, hook config format, prompt-based hooks recommendation, hook output contract, skill progressive disclosure, third-person description, imperative writing, command author intent, command frontmatter, MCP naming, MCP layout. **All gaps have concrete fixes baked into §16.2–16.6.** Layout/template are ready to scaffold in M0.
>
> **No new USER CHALLENGES from Phase 5** — best practices are non-controversial conformance items, all auto-decided. Decision audit trail rows 30-40 added.

---

## Phase 3.5 DX Review — SUBAGENT VOICE (2026-04-23)

> Note: Codex DX call hung in stdin mode (third codex invocation; prior two completed normally). Phase 3.5 proceeded with subagent-only voice. Source tag: `[subagent-only]`.

### DX Scorecard (subagent — vibe coder persona)

| Dimension (target) | Score |
|---|---|
| Getting started TTHW (≤5 min) | **3/10** — first-run UX 부재, binary 별도 install, ≥10분 현실 |
| Error message actionability | **3/10** — exit code mapping은 있으나 메시지 clinical, empathy 0 |
| NL ergonomics for non-engineers | **5/10** — §6.3 lexicon 시작은 좋으나 thin, deixis ("그거") 처리 X, clarify fallback X |
| Slash command discoverability | **2/10** — `/axhub` umbrella 없음, Korean alias 없음, `/help <topic>` 없음 |
| Korean docs completeness | **2/10** — README 1줄만, 다중 audience 구조 부재, GIF/video 부재 |
| Upgrade path safety | **4/10** — Phase 3 MIN_CLI_VERSION 추가됐으나 plugin self-upgrade nudge 없음 |
| Org-admin onboarding completeness | **2/10** — persona는 있으나 doc 미명세, rollout playbook 부재 |
| Fear management / trust building | **4/10** — AskUserQuestion + PreToolUse gate (Phase 3) 강하나, preview card / dry-run NL / empathy / forward-fix recover / status narration 모두 부재 |
| **Overall** | **25 / 80** — 안전 primitives는 강하지만 인간 surface가 비어있음 |

### Critical / High findings

**DX-1 critical — TTHW invisible.** §10/§11 M0–M6에 first-run UX 0. 새 사용자가 `/plugin install` → "내 앱 보여줘" → exit 65 → "/axhub:login" 안내 → 슬래시 모름 → 포기. **Fix: §5.1 First-run flow as M1 deliverable.**

**DX-2 critical — Error messages = exit-code dictionary, no empathy.** "토큰이 만료됐습니다. /axhub:login을 실행하세요." (clinical, mention slash). Vibe coder 11pm 데모 시나리오에서 식은땀. **Fix: §8.4 Korean error copywriting catalog (4-part: emotion + cause + action + button) as M1, not M5.**

**DX-3 high — NL lexicon §6.3 too thin for ambiguous Korean.** "그거 띄워줘" / "올리자" / "쏘자" / "내보내자" 처리 X. silent failure. **Fix: 3x lexicon 확장 + `skills/clarify/` fallback (numbered choices) + `skills/recent-context/` deixis resolver.**

**DX-4 high — Slash discoverability = 0.** `/axhub` umbrella 없음. Korean alias 없음. **Fix: `/axhub` menu + `/axhub:help <topic>` + Korean alias `/axhub:배포`.**

**DX-5 high — Korean docs = 1 line in §5.** Vibe-coder-quickstart vs senior reference vs org-admin policy 분리 X. GIF/asciinema 없음. **Fix: 4-doc structure as M1: README (admin) + vibe-coder-quickstart.ko + troubleshooting.ko + rollout-playbook.ko + 30-sec asciinema.**

**DX-6 high — Upgrade nudge invisible.** CLI v0.1.1 ship 시 vibe coder 모름. **Fix: SessionStart compares MIN_CLI_VERSION + RECOMMENDED_CLI_VERSION + plugin self-version, emits Korean upgrade prompt + `skills/upgrade/` (NL: "axhub 새 버전").**

**DX-7 critical (B2B blocker) — Org admin onboarding missing entirely.** Persona §1.5에 org admin 있으나 doc 0. 첫 customer company 도입 시 deal blocker. **Fix: `docs/org-admin-rollout.ko.md` (pre-rollout checklist + distribution + policy levers + incident runbook) as M6 — currently hand-waved.**

**DX-8 critical — Fear management features 거의 부재.** Preview card (profile/app/branch/commit/ETA in Korean) 없음. `--dry-run` NL trigger 없음. Empathetic failure 없음. Rollback `skills/recover/` (forward-fix-as-rollback) 없음. Status watch silent JSON tick (안심 narration 없음). **Fix: 새 §16 "Trust UX patterns" with all 5 elements.**

### Vibe coder empathy narrative (subagent)

> "밤 11시. 내일 사장님 데모. 결제 페이지 버그 발견. Cursor로 30분 만에 고치고 commit. 이제 axhub로 올려야 한다. Claude Code 켜고 '#paydrop 배포해줘' 침. 새 창 뜨고 '다른 배포가 진행 중. axhub deploy status로 그것부터 확인.' 무슨 말이야? 내가 뭘 잘못 누른 거야? ... 식은땀. ... 한 번 더 시도. 'paydrop 상태.' 이번엔 됐다. JSON 한 페이지. 글자만 가득. ... 다시 친다. '이제 올려도 돼?' '토큰이 만료됐습니다. /axhub:login을 실행하세요.' /가 뭐야? ... **포기. 시니어 깨운다. 사장님 데모 망함.**"

**Each friction point in this narrative maps to DX-1 ~ DX-8 findings.** Plan as currently written produces this outcome.

### Developer journey map (TTHW per current PLAN)

| Stage | Vibe coder action | Time | Friction | Priority |
|---|---|---|---|---|
| 1. Discover | Slack DM from senior | unknown | No discovery doc | High |
| 2. Install | `/plugin marketplace add` + `/plugin install` | ~2 min | Slash 모름, GIF X, binary 별도 | **Critical** |
| 3. Auth | exit 65 trigger or `/axhub:login` | ~1 min | Headless/Codespaces story X (Phase 3 E12), 한국어 instruction X | **Critical** |
| 4. Find app | "내 앱 보여줘" → `apps list` | ~10 sec | OK in PLAN | Medium |
| 5. Deploy | "paydrop 배포해" → AskUserQuestion → confirm | ~20 sec + 3 min build | Preview card X, generic consent, profile echo only in code | **Critical** |
| 6. Watch status | Auto-followed `--watch` | 3-5 min | Silent NDJSON, humanized progress X | High |
| 7. Hit error | exit 65/64 → PostToolUse classify | 5 sec | Robotic message, "everything is fine" framing X | **Critical** |
| 8. Recover | Read msg, follow instruction | 30 sec - 5 min | Recovery prose = jargon, auto-retry on benign X | High |
| 9. Re-deploy | Repeat stage 5 | 20 sec | OK if 1-8 fixed | Low |

**Current TTHW estimate: ≥10 min frictionless / 30-60 min realistic / OR abandon. Target ≤5 min unmet.**

### Phase 3.5 transition summary

> **Phase 3.5 complete.** Subagent-only (codex stdin issue). 8 findings (4 critical, 3 high, 1 medium). DX overall: 25/80. TTHW: 10-60min current → ≤5min target.
> 
> **2 architectural decisions surface as USER CHALLENGES** (Phase 4 final gate): (a) Empathy error catalog promoted to M1 (was M5 polish), (b) 4-doc Korean structure including org-admin rollout playbook (was 1-line hand-wave). Both are non-trivial scope expansions but Phase 3.5 makes them GO/NO-GO for B2B vibe coder adoption.
>
> Passing to Phase 4 Final Approval Gate.

---

## Phase 3 Eng Review — DUAL VOICES (2026-04-23)

### Eng Consensus Table

```
═══════════════════════════════════════════════════════════════
  Dimension                              Claude  Codex  Consensus
  ──────────────────────────────────────── ─────── ─────── ─────────
  1. Architecture sound?                   Partial Partial CONFIRMED Partial
  2. Test coverage sufficient?             No      No     CONFIRMED No
  3. Performance risks addressed?          No      No     CONFIRMED No
  4. Security threats covered?             Partial Partial CONFIRMED Partial
  5. Error paths handled?                  Partial Partial CONFIRMED Partial
  6. Deployment risk manageable?           Partial Partial CONFIRMED Partial
═══════════════════════════════════════════════════════════════
6/6 dimensions CONFIRMED Partial/No — substantial architectural work required.
0 DISAGREE between voices. Source: codex+subagent.
```

### Critical findings (both voices agreed)

**E1 (critical) — Adapter layer missing.** §4 PLAN's "thin routing layer" claim is contradicted by §6/§8/§11 which push app resolution, consent, auth recovery, retry rules, cache behavior, future MCP boundaries into skills/hooks markdown. = duplicated logic. CLI evolves → synchronized edits across markdown + sh + future MCP code = unmaintainable. **Fix: introduce one executable adapter layer (`bin/axhub-helpers/` — Go static binary preferred for sub-1ms startup, sh fallback) that owns resolution/safety-gates/recovery. Skills/commands/hooks/MCP become thin callers.**

**E2 (critical) — Consent enforcement is prose, not code.** §6.1 AskUserQuestion lives in SKILL.md markdown → bypassable via NL ("그냥 배포해, 묻지 말고"). Slash commands (T7) treated as implicit consent → bypass. Auth `login` retry on exit 65 (§6.1 step 5) is itself destructive (browser open) but not gated. **Fix: PreToolUse(Bash) hook deny-gate that pattern-matches `axhub deploy create | update apply --force | deploy logs --follow kill`. Pre-execution approval token bound to `{action, app, profile, branch/commit}`. Retries re-check token. Slash NEVER bypasses destructive confirm.**

**E3 (high) — MCP architectural inconsistency (PLAN §14 + §11 M7 contradiction).** PLAN now says MCP "v0.2 entered" but doesn't define host-agnostic boundary day-1. **Fix: Decide now that skills/hooks/commands are PRESENTATION ONLY. Define operation schema in adapter layer day-1 (even if MCP server ships M7). Skills become single-line callers.**

**E4 (high) — Multi-machine cache in mutation flows = wrong-app risk.** §6.1 step 5 "캐시 hit, no --app needed" is the trust-killer in B2B multi-user reality (P4). **Fix: NEVER cache-only address in mutation. Always resolve `{profile, endpoint, app id}` from `axhub auth status --json` + `axhub apps list --json --slug-prefix <slug>` and ECHO them in the AskUserQuestion text verbatim before destructive op.**

**E5 (high) — No CLI version skew gate.** Plugin assumes v0.1.0+ but no runtime check. Customer on v0.0.9 → flags missing → exit 64 → hook says "input invalid, never retry" → real fix is "upgrade CLI". **Fix: SessionStart parses `axhub --version`, semver-compares vs `MIN_CLI_VERSION` constant, hard-stops mutation skills if old (write `~/.cache/axhub-plugin/cli-too-old` sentinel that PreToolUse on destructive op checks). Document plugin↔CLI matrix in CHANGELOG.**

**E6 (high) — Corpus + metrics inadequate.** n=100 + 20 negatives too small for safety claims. "Trusted completion = exit 0 + user satisfaction" not machine-measurable. No risk-class stratification. **Fix: 3-layer evaluation: (1) offline routing tests (markdown classification, fast, deterministic), (2) contract tests (helper layer fixtures), (3) much smaller e2e acceptance suite (~20 cases). Stratify by risk: read-only / mutation / safety. Adversarial negative set ≥40.**

**E7 (high) — Token/secret model = documentation only.** No concrete model for: per-machine login lifecycle, token-file 0600 enforcement, shared-machine policy, redaction enforcement. **Fix: Strict secret model in adapter layer: never echo token-file paths, redact `apis list` `service_base_url` of cross-team APIs, enforce per-machine OAuth (no token sync), explicit headless/Codespaces flow (token-file paste).**

**E8 (medium) — Hook 5ms gate unrealistic + ordering undefined.** sh + jq cold start = 20-60ms. Hook order across plugins undefined. **Fix: Realistic gate = 50ms p95, OR Go binary in `bin/` (sub-1ms). Pure-sh early-return for non-Bash + non-axhub events before any JSON parse. Hook payload contract pinned with fixtures.**

**E9 (medium) — AskUserQuestion fatigue, no consent state machine.** Auth + destructive + watch/follow prompts nest ad hoc. **Fix: One consent state machine in adapter layer with dedupe (same `{action, app, profile}` within N seconds = 1 prompt), expiry, escalation rules.**

**E10 (high) — CLI contract drift already present.** §6.1 step 3 references `axhub deploy list --status building` but §3.1 matrix has NO `deploy list` command. Plan speculating beyond verified CLI. **Fix: VERIFY `deploy list` exists in v0.1.0 GA before M1. If absent, delete pre-flight, document race detection as exit-64-only, add T9 assertion accordingly.**

### Subagent-only critical additions

**E11 (critical) — Hook stdin contract unspecified.** §8.2 `classify-exit.sh` reads `tool_input.command` and `tool_response.exit_code` but Claude Code's actual PostToolUse JSON shape for Bash (stdout/stderr/interrupted/isImage) never pinned. Day 1 of M4 → hook errors on EVERY Bash call across the user's session. **Fix: top-of-script `jq -e '.tool_name == "Bash"' >/dev/null || exit 0` guard, pin shape in §8 with annotated example, emit via `{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"…"}}`.**

**E12 (medium) — Headless/Codespaces auth missing.** OAuth Device Flow needs browser; vibe coder on Codespaces stuck. **Fix: `skills/auth/SKILL.md` detect `$CODESPACES` / no-DISPLAY / no-`open` → present token-file paste flow.**

**E13 (medium) — apis list privacy/scope leak.** Broad-scope token surfaces other-team APIs. **Fix: filter `--app-id $CURRENT_APP` by default, AskUserQuestion before cross-team list.**

**E14 (medium) — SSE mid-stream + cosign failure paths zero tests.** Add T11 (SSE Last-Event-ID resume) + T12 (cosign failure hard-stop, no fallback to unsigned).

**E15 (medium) — Hook ordering race.** PostToolUse fires after user already typed next prompt. **Fix: state files (`~/.cache/axhub-plugin/last-exit.json`) read by next UserPromptSubmit instead of context injection race.**

### Architecture diagram (Eng subagent — critical breakage points marked)

```
User (vibe coder)
   │ "paydrop 배포해"
   ▼
Claude Code harness
   ├─ UserPromptSubmit hook (intent-detect.sh)        ⚠ 5ms gate unrealistic (E8)
   │
   ▼
Skill router (description matching) ← AskUserQuestion ⚠ markdown-only enforcement (E2)
                                    ← profile echo    ⚠ not coded (E4)
   │ Bash tool
   ▼
PreToolUse(Bash) hook ⚠ MISSING — required for E2 deny-gate
   │
   ▼
Bash subshell ── stdout/stderr ──▶ PostToolUse hook ⚠ stdin shape unpinned (E11)
   │
   ▼
axhub CLI binary ⚠ MIN_CLI_VERSION not enforced (E5)
   │
   ├──▶ OS keychain (per-machine, ⚠ no Codespaces story E12)
   ├──▶ ~/.config/axhub/deployments.json cache (per-machine, ⚠ E4 mutation risk)
   └──▶ https://hub-api.jocodingax.ai
              ▲
              │
M7 (v0.2): MCP server ⚠ unbuilt; coupling risk if skills/ stay markdown-only (E1, E3)
```

### Phase 3 transition summary

> **Phase 3 complete.** Codex: 12 concerns. Claude subagent: 12 findings + ASCII arch + 28-row test matrix + 14-row failure modes registry. Consensus: 6/6 dimensions CONFIRMED Partial/No. 0 disagreements between voices. 
> 
> **3 architectural decisions surface as USER CHALLENGES** (will go to Phase 4 final gate): (1) Add adapter layer `bin/axhub-helpers/` (added complexity vs trust requirement), (2) PreToolUse deny-gate for ALL destructive ops (slash bypass elimination), (3) Day-1 host-agnostic helper contract (MCP-aware architecture even before M7).
> 
> Passing to Phase 3.5 (DX Review).

---

## Decision Audit Trail (autoplan 진행 시 채워짐)

<!-- AUTONOMOUS DECISION LOG -->

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|---|---|---|---|---|---|
| 1 | Phase 0 | Plugin name = `axhub` | Mechanical | P5 explicit | CLI 이름과 통일, 짧음 | ax-hub, axhub-cc |
| 2 | Phase 0 | Target contract = v0.1.0 GA | Mechanical | P1 completeness | 오늘 GA, agent contract 완성 | v0.0.x only, dual stub |
| 3 | Phase 0 | MCP = NOT in scope | **REVERSED in Phase 1** | — | (was: CLI agent-native) | — |
| 4 | Phase 0 | Hooks = SessionStart + PostToolUse | Mechanical | P2 boil lakes | exit code 자동 분류 가치 명확 | hooks 없음 |
| 5 | Phase 1 | Plugin scope = B2B vibe-coder hub plugin | **User decision** | (user clarification) | 사용자 컨텍스트로 모호성 해소 | Internal tool, Generic marketplace, Plugin defer |
| 6 | Phase 1 | NL metric = 4-metric replacement | **User decision** | C2 (CEO consensus) | trusted-completion + unsafe-trigger 0% + recovery + baseline-delta | 90% trigger 유지, 보강 |
| 7 | Phase 1 | MCP = v0.2 (M7) entered | Auto (P9) | C4 (CEO consensus) | cross-agent portability 가치 | MCP 영영 안 함 |
| 8 | Phase 1 | M0.5 docs-only baseline + M1.5 GO/KILL gate | Auto (P10) | C1 (CEO consensus) | plugin existence falsifiable | baseline 없이 진행 |
| 9 | Phase 1 | AskUserQuestion before destructive ops | Auto (P7) | C3 (CEO consensus) | trust model 1순위 | 기본 trigger만 |
| 10 | Phase 3 | **Adapter layer `bin/axhub-helpers/` required** | **USER CHALLENGE** | E1 (Eng consensus) | Logic이 markdown 분산 = 유지보수 불가 | markdown-only thin routing |
| 11 | Phase 3 | **PreToolUse deny-gate for ALL destructive (incl. slash)** | **USER CHALLENGE** | E2 (Eng consensus) | consent enforcement이 prose면 bypass 가능 | slash = implicit consent (현 PLAN) |
| 12 | Phase 3 | **Day-1 host-agnostic helper contract** | **USER CHALLENGE** | E3 (Eng consensus) | MCP 추가 시 재작성 방지 | MCP는 M7에 별도 작성 |
| 13 | Phase 3 | Live profile/app resolve in mutation (no cache) | Auto | E4, P4 | wrong-app/profile 위험 | cache 우선 |
| 14 | Phase 3 | MIN_CLI_VERSION semver gate at SessionStart | Auto | E5, P8 | silent bad behavior 방지 | print-only |
| 15 | Phase 3 | 3-layer eval (offline routing + contract + e2e) | Auto | E6 | corpus runner 비용/신뢰 | n=100 단일 layer |
| 16 | Phase 3 | Hook gate = 50ms p95 (or Go binary) | Auto | E8 | sh+jq 현실 | 5ms 유지 (불가능) |
| 17 | Phase 3 | Hook stdin contract pinned with fixtures + non-Bash early-return | Auto | E11 (subagent critical) | day-1 break 방지 | unpinned |
| 18 | Phase 3 | Verify `deploy list` exists in v0.1.0 GA before M1 | Auto | E10 | speculative dep 제거 | assume exists |
| 19 | Phase 3.5 | Empathy error catalog as M1 deliverable (Korean, 4-part: emotion/cause/action/button) | **USER CHALLENGE** | DX-2 (subagent critical) | 11pm 데모 narrative에서 plugin abandonment 입증 | M5에 미루기 |
| 20 | Phase 3.5 | Pre-deploy preview card before AskUserQuestion (profile + app + branch + commit + ETA in Korean) | Auto | DX-8, E4 | "다음을 실행할게요" UX, fear management 핵심 | data echo만 |
| 21 | Phase 3.5 | SessionStart first-run flow (3-step Korean welcome) | Auto | DX-1 critical | TTHW 8-12분 → ≤5분 목표 | M6에 미루기 |
| 22 | Phase 3.5 | `skills/clarify/` no-skill-matched fallback | Auto | DX-3 | silent failure 차단, "그거 띄워줘" 처리 | 자연 fail |
| 23 | Phase 3.5 | `skills/recover/` forward-fix-as-rollback | Auto | DX-8 | rollback 미지원 + "방금 거 되돌려줘" 자연 발화 | rollback 안 함 |
| 24 | Phase 3.5 | NL lexicon §6.3 3x 확장 + deixis resolver (~/.config/axhub/recent.json) | Auto | DX-3 | "올리자/쏘자/내보내자/그거" 처리 | 현 lexicon |
| 25 | Phase 3.5 | `/axhub` umbrella menu + Korean aliases (`/axhub:배포` 등) | Auto | DX-4 | discoverability 0 → menu 1 | 영어만 |
| 26 | Phase 3.5 | 4-doc Korean structure (vibe-coder-quickstart + troubleshooting + org-admin-rollout + README) | **USER CHALLENGE** | DX-5, DX-7 critical | B2B에 org admin doc 부재는 deal blocker | README 1개만 |
| 27 | Phase 3.5 | `--dry-run` NL triggers ("한번 해보기만", "리허설", "테스트로") | Auto | DX-8 | preview 후 진짜 실행 전 마지막 escape | flag만, NL 안 함 |
| 28 | Phase 3.5 | Plugin self-upgrade nudge in SessionStart + `skills/upgrade/` | Auto | DX-6 | CLI/plugin 둘 다 stale → silent bad behavior | print만 |
| 29 | Phase 3.5 | Status watch with humanized Korean progress narration ("1분 경과, 빌드 중이에요 (정상)") | Auto | DX-8 | 침묵 JSON tick = 불안 증폭 | 그대로 NDJSON |
| 30 | Phase 5 | All hook commands use `${CLAUDE_PLUGIN_ROOT}` prefix | Auto | G1 official | install 위치 portability | 절대경로 |
| 31 | Phase 5 | hooks.json `{"hooks": {...}}` wrapper format | Auto | G2 official | Plugin format 필수, load 실패 방지 | 직접 top-level |
| 32 | Phase 5 | PreToolUse deny-gate = prompt-based hook (대신 sh script도 호환) | Auto | G3 official | context-aware reasoning, sh보다 우월 | sh script만 |
| 33 | Phase 5 | Hook outputs `hookSpecificOutput.permissionDecision` JSON | Auto | G4 official | Claude가 결정 인식 | exit code만 |
| 34 | Phase 5 | Skill progressive disclosure (SKILL.md ≤2000w + references/ + examples/ + scripts/) | Auto | G5 official | context bloat 방지, 한국어 detail은 references/에 | SKILL.md 무제한 |
| 35 | Phase 5 | Skill description = third-person + specific trigger phrases | Auto | G6 official | auto-discovery 강화 | 1인칭/vague |
| 36 | Phase 5 | Skill body = imperative/infinitive form (no second-person) | Auto | G7 official | AI consumption 표준 | "you should..." |
| 37 | Phase 5 | Commands written as instructions FOR Claude (not messages TO user) | Auto | G8 official | command 본질 | user-facing copy |
| 38 | Phase 5 | Command frontmatter: description + allowed-tools + argument-hint + model | Auto | G9 official | security/UX 표준 | description만 |
| 39 | Phase 5 | MCP tool naming `mcp__plugin_axhub_axhub-agent__<tool>` 미리 결정 | Auto | G10 official | M7 매끄러운 진입 | 미정 |
| 40 | Phase 5 | `.mcp.json` placeholder at plugin root for M7 | Auto | G11 official | recommended layout | inline |
| 41 | Phase 5 | Adapter layer = `bin/axhub-helpers/` Go binary, on PATH via plugin enable | Auto | E1 + G1 | 공식 `bin/` mechanism | sh scripts |
| 42 | Phase 5 | Pre-M1 best practices audit (skill-reviewer + plugin-validator + claude --debug) | Auto | §16.7 | gap 0 보장 | 직접 review |
| 43 | Phase 6 | **REVERSE row 32**: PreToolUse = command-based Go hook (HMAC consent) | **USER CHALLENGE** | F1 (4 voices CRIT) | LLM에 보안 결정 = bag-of-words bypass | prompt-based 유지 |
| 44 | Phase 6 | Mutation path always live-resolve `{app_id, profile, endpoint, branch, commit}` + echo all 5 | Auto | F2 (3 voices) | wrong-app deploy = trust death | cache OK |
| 45 | Phase 6 | Per-customer keychain namespacing + shared-machine policy | Auto | F3 (3 voices CRIT) | multi-tenant leak | global keychain |
| 46 | Phase 6 | apis list default = current-team scope, cross-team = explicit consent + audit | Auto | F4 (3 voices CRIT) | regulated customer audit blocker | broad scope |
| 47 | Phase 6 | Plugin manifest cosign signed + multi-arch helper binary + macOS notarized | **USER CHALLENGE** | F5 (Adv P0 + Validator HIGH) | RCE-by-design via auto-update + Gatekeeper block | 미서명 |
| 48 | Phase 6 | **REVERSE row 26 ordering**: Org admin rollout playbook = M0 prereq, NOT M6 | **USER CHALLENGE** | F6 (Adv P0) | "no rollout doc → no marketplace" | M6 deferral |
| 49 | Phase 6 | CLI version: hard MIN + MAX + capability flags via `axhub --version --json` | Auto | F7 | min-only fails on v0.2.0 silent breakage | min only |
| 50 | Phase 6 | Adversarial corpus ≥200 stratified + frozen model/temp/version + 3-run CI | Auto | F8 | n=40 = statistical noise on 0% gate | n=40 |
| 51 | Phase 6 | `.mcp.json` add `mcpServers` wrapper | Auto | F9 (Validator CRIT) | silent fail to register | 잘못된 schema |
| 52 | Phase 6 | hooks.json SessionStart `matcher` 제거 | Auto | F10 (Validator CRIT) | invalid for SessionStart event | "matcher": "*" |
| 53 | Phase 6 | SKILL.md frontmatter: `name: <kebab>` only, drop `allowed-tools` | Auto | F11 (Validator HIGH) | hallucinated field | name spaces, allowed-tools |
| 54 | Phase 6 | Concrete plugin.json + marketplace.json schemas in §16.12 | Auto | F12 (Validator HIGH) | marketplace 등록 실패 | schema 미정 |
| 55 | Phase 6 | `bin/axhub-helpers` = single multi-cmd binary (not dir) | Auto | F13 (Validator HIGH) | hooks invoke as binary | dir vs binary 모호 |
| 56 | Phase 6 | Unicode hardening: NFKC normalize + Punycode display + Bidi/ZWJ filter | Auto | F14 (Sec) | Cyrillic homoglyph 공격 | 무방어 |
| 57 | Phase 6 | Hook schema versioning (fixtures v0/v1) + state files (no ordering dependency) | Auto | F15 (Adv + Codex) | harness payload drift = full session 깨짐 | unpinned |
| 58 | Phase 6 | MCP server (M7) embeds consent in MCP tools (`request_consent` + `consent_token` param), fail-closed | Auto | F16 (Sec HIGH) | cross-agent (Codex/Cursor) skip Claude Code hooks | host hook only |
| 59 | Phase 6 | Default `AXHUB_REQUIRE_COSIGN=1` (override = `AXHUB_ALLOW_UNSIGNED=1` with Korean warning) | Auto | Sec HIGH #8 | MITM swap binary + checksums | opt-in cosign |
| 60 | Phase 6 | TLS pinning for hub-api.jocodingax.ai SPKI hash (override = AXHUB_ALLOW_PROXY=1) | Auto | Sec defense-in-depth | corporate MITM proxy 구별 불가 | 무 pinning |
| 61 | Phase 6.5 | **CANCEL row 7 + 12 + 39 + 40 + 58**: M7 plugin MCP server, cross-agent portability, .mcp.json placeholder, MCP tool naming, MCP consent enforcement | **User clarification (2nd)** | "plugin이 MCP를 쓰는게 아니라 cli를 쓰는거야" | Plugin이 자체 MCP 서버 expose 또는 MCP 호출 = 사용자 의도 아님. backend의 CLI-replaceable MCP 기능만 ax-hub-cli로 마이그레이션. Plugin은 ax-hub-cli만 호출 | M7 MCP server (Phase 1/5/6 모두 잘못 추가) |
| 62 | Phase 6.5 | §11 milestones 재정렬: M7 삭제. M0~M6만 유지. v0.2는 별도 PLAN | Auto (cascade from #61) | row 61 | M7 (plugin MCP server)이 빠지면 milestone list 정정 | M7 유지 |
| 63 | Phase 6.5 | "Day-1 host-agnostic helper contract" (row 12) 의미 변경: cross-agent용 X, 단지 testable Go API design (skill markdown은 thin caller, helper는 CLI 호출 + state 관리) | Auto (cascade) | row 61 | Helper layer는 여전히 가치 (E1 maintainability) but 이유는 cross-agent 아니라 testability/maintainability | row 12 그대로 |
| 64 | Phase 6.5 | §14 NOT in scope에 명시: "Plugin이 자체 MCP server expose / MCP 호출" — v0.x 전체 영구 제외 (사용자 의도) | Auto | row 61 | 미래에 backend MCP가 잔존해도 plugin은 항상 CLI 통과 | "v0.2 MCP 검토" |
| 65 | Phase 6.5 | §16.16 multi-tenant credential isolation, §16.17 apis list privacy, §16.11 Unicode hardening, §16.14 hook schema versioning, §16.18 adversarial corpus 200+, §16.9 supply chain, §16.10 cosign default-on, §16.12 schemas, §16.13 single binary — **모두 유지** (MCP와 무관, 모든 finding 유효) | Auto | F2-F15 | MCP cancellation은 보안/품질 fix와 무관 | 함께 cancel |
| 66 | M0 | **REVERSE row 41 + 55 (helper language)**: bin/axhub-helpers = TypeScript on Bun runtime (was Go), single binary via `bun build --compile` | **User decision** | "ts 쓰는게 더 좋지 않아" | (1) Claude Code 자체가 Node/Bun 기반 (2) jax-plugin-cc도 .mjs 사용 (3) Bun cold start 5-20ms는 새 50ms hook gate에 충분 (Go 1ms는 over-engineering) (4) npm 생태계 (jose for HMAC, semver, zod) (5) vibe coder가 codebase 읽기 쉬움 (TS ≫ Go 친숙도) | Go 유지 |
| 67 | M0 | Build pipeline: Bun + npm scripts + `bun build --compile --target=bun-{platform}-{arch}`, multi-arch single binary, cosign-signed in CI | Auto (cascade) | row 66 | Bun이 멀티플랫폼 single binary 지원 (Go cross-compile 우월성 동등 cover) | go build / Makefile |
| 68 | M0 | Distribution: src/* TypeScript checked in, bin/axhub-helpers* gitignored (build artifact), runtime requirement = Bun ≥1.1 (개발자) / 사용자는 pre-built binary 사용 | Auto | row 66 | Bun runtime은 dev 머신에만 필요. 사용자는 release tarball의 pre-compiled binary 다운로드 | source-in-bin |

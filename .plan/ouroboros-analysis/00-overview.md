# Ouroboros 플러그인 전체 분석 — Overview

> 분석 대상: https://github.com/Q00/ouroboros.git (main HEAD)
> 분석 방법: ralplan consensus (Planner → Architect → Critic)
> 분석 날짜: 2026-04-29
> 패키지 버전: 0.30.0 (PyPI: ouroboros-ai)

## 한 줄 요약

Ouroboros = Specification-first AI coding workflow engine. Socratic interview 로 ambiguity ≤ 0.2 까지 정제 → immutable Seed 생성 → Double Diamond 분해 + 병렬 실행 → 3-stage evaluation → Wonder/Reflect 진화 루프. Claude Code/Codex/OpenCode/Hermes 4개 런타임 동등 추상화. Event-sourced SQLite 영속성으로 stateless cycle 가능.

## Plugin Identity

| 속성 | 값 |
|---|---|
| 이름 | `ouroboros` (PyPI: `ouroboros-ai`) |
| 버전 | 0.30.0 |
| 라이선스 | MIT |
| 저자 | Q00 |
| Python | >= 3.12 |
| Rust | >= 1.74 (TUI 크레이트) |
| 진입점 | `ouroboros = "ouroboros.cli.main:app"` (Typer, 12 서브그룹) |
| MCP | `uvx --from ouroboros-ai[mcp,claude] ouroboros mcp serve` |
| 슬래시 | `/ouroboros:<skill>` (13 commands, 19 skills) — CLI 진입점은 12 서브그룹 (Section 14 참조) |

## 6-Phase Pipeline

```
Phase 0: BIG BANG          Interview → Seed (ambiguity ≤ 0.2 게이트)
Phase 1: PAL ROUTER        Frugal 1× → Standard 10× → Frontier 30×
Phase 2: DOUBLE DIAMOND    Discover → Define → Design → Deliver (재귀, DEFAULT_MAX_DECOMPOSITION_DEPTH=2)
Phase 3: RESILIENCE        4 stagnation 패턴 + 5 lateral persona
Phase 4: EVALUATION        Mechanical $0 → Semantic $$ → Consensus $$$
Phase 5: SECONDARY LOOP    비-블로킹 TODO 배치 처리

(↺ Evolutionary Loop: Wonder → Reflect → 다음 세대 시드, similarity ≥ 0.95 까지, hard cap 30)
```

## 통계

- 총 파일: 724
- Python 소스: 259 (.py)
- Skills: 19 (총 3941 LOC)
- Commands: 13 (.md stubs)
- Bundled agents: 21 (.md personas)
- Scripts: 10 (3 hooks + install + version + ralph 유틸)
- Docs: 35
- Tests: 270+ 파일, 7797+ LOC
- Rust TUI: 별도 crate (`crates/ouroboros-tui/`)
- TS bridge: `src/ouroboros/opencode/plugin/ouroboros-bridge.ts` (560 LOC)

## 4 Runtime Backends

| Backend | 구현 모듈 | 기반 | 특징 |
|---|---|---|---|
| claude | `orchestrator/adapter.py` (1595 LOC) | claude-agent-sdk + Claude Code CLI | 가장 정교, MCP delegation hook, shared rate limit bucket, 3 retry exponential backoff |
| codex | `orchestrator/codex_cli_runtime.py` | OpenAI Codex CLI subprocess | NDJSON parser, skill-command interception |
| opencode | `orchestrator/opencode_runtime.py` | OpenCode CLI | subprocess 모드 강제 (plugin 모드 = MCP 서버 컨텍스트 전용) |
| hermes | `orchestrator/hermes_runtime.py` | Hermes CLI | 가장 신규 |

## 4 Hook 시스템

`hooks/hooks.json` 등록:

| Hook | 스크립트 | 역할 |
|---|---|---|
| SessionStart | `scripts/session-start.py` | 24h 캐시된 PyPI 버전 체크 (stderr) |
| UserPromptSubmit | `scripts/keyword-detector.py` | 28 키워드 패턴 매칭 → 스킬 라우팅, MCP 미설치 시 setup 강제 |
| PostToolUse(Write\|Edit) | `scripts/drift-monitor.py` | 1시간 내 활성 인터뷰 세션 감지 → drift 안내 |

## 분석 문서 구조

| 파일 | 내용 |
|---|---|
| `00-overview.md` | 이 문서 |
| `01-identity-and-distribution.md` | Plugin manifest, 배포 채널, optional extras, hooks |
| `02-directory-topology.md` | 27 카테고리 디렉토리 매핑, 파일 통계 |
| `03-phase-0-bigbang.md` | Interview → Seed 흐름, ambiguity 수식, PATH 1a/1b/2/3/4, Dialectic Rhythm Guard |
| `04-phase-1-pal-router.md` | 3-tier 비용 라우팅, complexity 수식, escalation/downgrade |
| `05-phase-2-double-diamond.md` | 재귀 분해, parallel_executor, runner, coordinator |
| `06-phase-3-resilience.md` | 4 stagnation 패턴, 5 페르소나, recovery |
| `07-phase-4-evaluation.md` | 3-stage pipeline, 6 consensus trigger, AC-agnostic invariant |
| `08-phase-5-secondary.md` | TODO registry, batch scheduler |
| `09-evolutionary-loop.md` | Wonder/Reflect, convergence, ralph |
| `10-persistence.md` | EventStore, schema, checkpoints, UoW |
| `11-runtime-abstraction.md` | AgentRuntime Protocol, RuntimeHandle, 4 어댑터 비교 |
| `12-mcp-hub.md` | Bidirectional MCP, 노출 도구, 에러 트리 |
| `13-plugin-skills-agents.md` | 19 skills, 21 agents, plugin infra |
| `14-cli-surface.md` | Typer 진입점, 13 서브 그룹, formatters |
| `15-tui-implementations.md` | Python Textual + Rust SuperLightTUI |
| `16-opencode-bridge.md` | TS plugin, fire-and-forget dispatch |
| `17-hooks-detail.md` | 3 hook 동작 상세 |
| `18-config-filesystem.md` | `~/.ouroboros/` 레이아웃, 환경 변수, 보안 한계 |
| `19-quality-ci-release.md` | ruff/mypy/pytest, GitHub Actions, 릴리즈 흐름 |
| `20-tests-layout.md` | 270+ 테스트 디렉토리 매핑 |
| `21-architect-review.md` | 강점, 트레이드오프, synthesis |
| `22-critic-evaluation.md` | 채점 + verdict |
| `23-final-adr.md` | Decision, drivers, consequences, follow-ups |
| `24-deep-dive-entrypoints.md` | 추가 조사 시 진입 매핑 |
| `25-magic-numbers.md` | 핵심 상수 단일 출처 표 (MAX_DEPTH 5→2 정정) |
| `26-dependencies.md` | 외부 의존성 인벤토리 |
| `27-confirmation-gaps.md` | 확인 못 한 영역 정직하게 마킹 (이번 ralph round 의 채워진 gap 마킹) |
| `28-deep-runner-internals.md` | runner.py 2722 LOC + parallel_executor.py 3479 LOC + init.py 855 LOC body |
| `29-small-modules.md` | Wonder/Reflect/Regression/Verifier/Scheduler/InputValidator/Escalation/Downgrade |
| `30-external-reviews.md` | Code-Review-Claude vs Code-Review-Codex (Hermes 통합 PR) |
| `31-governance.md` | SECURITY + HANDOFF + UNINSTALL + CONTRIBUTING + ISSUE_TEMPLATE |
| `32-docs-deep-dive.md` | 33 doc 파일 정독 (getting-started / 10 guides / 5 contributing / 3 api / 2 examples) |
| `33-changelog-history.md` | 0.1.0 ~ 0.14.1 milestone + Removed (StateStore/StateManager/RecoveryManager/StateCompression) |
| `34-tui-permissions-bridge.md` | Round 2 review gap-finder 발견 — Rust TUI views + Python widgets/screens + claude/codex_permissions.py + MCPBridge body |
| `35-residual-modules.md` | Round 3 gap fill — parallel_executor_models (AC outcome enum 본체) + execution_runtime_scope + verification/extractor + MCPClientManager |
| `INDEX.md` | 35+ section navigation hub (topic / phase / file path 별 진입, file 라벨 정정 후) |

## 핵심 발견 5선

1. **Math-gated phases** — 직관 게이트 대신 ambiguity ≤ 0.2, similarity ≥ 0.95, drift > 0.3, complexity 0.4/0.7 모두 코드+docs 일치
2. **Bidirectional MCP Hub** — 서버 + 클라이언트 + server-to-server bridge
3. **4 runtime parity attempt** — Protocol + alias normalization 한 곳 (`_RUNTIME_HANDLE_BACKEND_ALIASES`)
4. **Event-sourced everything** — TUI/CLI/MCP/Rust TUI 모두 동일 SQLite 폴링
5. **Self-referential loop** — Wonder/Reflect 가 이름값 (재시도 ≠ 진화)

## 핵심 위험 3선

1. `mypy disable_error_code` 14개 → 사실상 타입 검사 없음
2. `runner.py` 109K + `parallel_executor.py` 144K → 거대 단일 파일, 인지 부하
3. `evaluation/pipeline.py:113-122` Stage 1 AC-agnostic invariant fragile (주석으로만 경고)

## 다음 단계

전체 navigation 은 `INDEX.md` 참조 — topic/phase/file path 별 진입 가이드. Deep-dive entrypoint 는 `24-deep-dive-entrypoints.md`. 미독 영역 추적은 `27-confirmation-gaps.md`.

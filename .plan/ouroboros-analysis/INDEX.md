# Ouroboros Plugin Analysis — INDEX

> 35 section (00-34) + supporting docs 의 navigation hub. **task / phase / topic** 별로 어느 문서를 보아야 하는지 가이드.

## 빠른 시작 (어디서 시작?)

| 목적 | 시작 문서 |
|---|---|
| Ouroboros 가 뭔지 (5분) | [00-overview.md](./00-overview.md) |
| Phase 0~5 흐름 (15분) | [03](./03-phase-0-bigbang.md) → 04 → 05 → 06 → 07 → 08 순서 |
| 특정 phase 깊이 | 해당 Section (03 Big Bang / 04 PAL Router / 05 Double Diamond / 06 Resilience / 07 Evaluation / 08 Secondary Loop / 09 Evolutionary Loop) |
| 외부 통합 (MCP / Hermes / OpenCode) | 12 / 16 / 11 |
| 4 backend 비교 | [11-runtime-abstraction.md](./11-runtime-abstraction.md) |
| 운영자 가이드 | 31 (governance) → 19 (CI) → 32 (docs) |
| 코드 상수 사냥 | [25-magic-numbers.md](./25-magic-numbers.md) |
| 미독 영역 추적 | [27-confirmation-gaps.md](./27-confirmation-gaps.md) |

## 35 Section 카탈로그 (실제 file 라벨)

### Foundation (00–02)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 00 | `00-overview.md` | Overview | 프로젝트 metadata + 6-Phase pipeline 요약 + 분석 문서 구조 |
| 01 | `01-identity-and-distribution.md` | Plugin Identity & Distribution | manifest, 배포 채널, optional extras, hooks |
| 02 | `02-directory-topology.md` | Directory Topology | 27 카테고리 디렉토리 매핑, 파일 통계 |

### Phase Pipeline (03–10)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 03 | `03-phase-0-bigbang.md` | Phase 0 — BIG BANG (Interview → Seed) | ambiguity ≤ 0.2 게이트 + Dialectic Rhythm Guard |
| 04 | `04-phase-1-pal-router.md` | Phase 1 — PAL Router (Progressive Adaptive LLM) | 3 tier (Frugal/Standard/Frontier) + escalation/downgrade |
| 05 | `05-phase-2-double-diamond.md` | Phase 2 — Double Diamond (Execution) | parallel_executor + AC outcome 5-way + Recovery flow |
| 06 | `06-phase-3-resilience.md` | Phase 3 — Resilience (Stagnation + Lateral Thinking) | stagnation 4 패턴 + 5 lateral persona |
| 07 | `07-phase-4-evaluation.md` | Phase 4 — Evaluation (3-Stage Pipeline) | Mechanical + Semantic + Consensus |
| 08 | `08-phase-5-secondary.md` | Phase 5 — Secondary Loop | TODO registry + batch scheduler |
| 09 | `09-evolutionary-loop.md` | Evolutionary Loop (Wonder / Reflect Cycle) | Wonder/Reflect + convergence + ralph |
| 10 | `10-persistence.md` | Persistence — Event Sourcing Layer | SQLite EventStore + replay + checkpoint |

### Surface (11–17)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 11 | `11-runtime-abstraction.md` | Runtime Abstraction Layer | AgentRuntime Protocol + 4 backend (Claude/Codex/OpenCode/Hermes) |
| 12 | `12-mcp-hub.md` | MCP Hub (Bidirectional) | 23 MCP tool catalog (server + client + bridge) |
| 13 | `13-plugin-skills-agents.md` | Plugin / Skills / Agents Subsystem | 19 skill + 21 agent persona + 13 command stub |
| 14 | `14-cli-surface.md` | CLI Surface (Typer) | Typer CLI + PM seed auto-detect + force-bypass |
| 15 | `15-tui-implementations.md` | TUI Implementations (Python + Rust) | Textual Python + SuperLightTUI Rust |
| 16 | `16-opencode-bridge.md` | OpenCode TS Bridge Plugin | TS plugin (560 LOC) — fire-and-forget dispatch |
| 17 | `17-hooks-detail.md` | Hooks System Detail | 4 hook (SessionStart / UserPromptSubmit / PostToolUse) |

### Quality + Operations (18–20)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 18 | `18-config-filesystem.md` | Configuration & Filesystem | `~/.ouroboros/` 레이아웃, 환경 변수, 보안 한계 |
| 19 | `19-quality-ci-release.md` | Quality / CI / Release | ruff/mypy/pytest + release.yml + dev-publish.yml |
| 20 | `20-tests-layout.md` | Tests Layout | 270+ test file + invariant 검증 detail |

### Review + ADR (21–24)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 21 | `21-architect-review.md` | Architect Review | 강점, 트레이드오프, synthesis |
| 22 | `22-critic-evaluation.md` | Critic Evaluation | 채점 + verdict |
| 23 | `23-final-adr.md` | Final Plan ADR | Decision, drivers, alternatives, consequences |
| 24 | `24-deep-dive-entrypoints.md` | Deep-dive Entrypoints | 추가 조사 시 진입 매핑 |

### Reference (25–27)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 25 | `25-magic-numbers.md` | Magic Numbers — Single Source of Truth | 모든 임계값 / 한도 / 상수 (MAX_DEPTH 5→2 정정) |
| 26 | `26-dependencies.md` | Dependencies Inventory | 외부 의존성 |
| 27 | `27-confirmation-gaps.md` | Confirmation Gaps | 미독 영역 tracker (이번 round 채운 gap ✅) |

### Deep-dive — Ralph round 추가 (28–34)

| # | 파일명 | H1 라벨 | 핵심 |
|---|---|---|---|
| 28 | `28-deep-runner-internals.md` | Deep Runner Internals | runner.py 2722 + parallel_executor 3479 + init.py 855 LOC |
| 29 | `29-small-modules.md` | Small Modules | Wonder/Reflect/Regression/Verifier/Scheduler/InputValidator/Escalation/Downgrade |
| 30 | `30-external-reviews.md` | External Reviews — Claude vs Codex | Hermes 통합 audit 비교 |
| 31 | `31-governance.md` | Governance | SECURITY + HANDOFF + UNINSTALL + CONTRIBUTING + ISSUE_TEMPLATE |
| 32 | `32-docs-deep-dive.md` | Docs Deep-Dive | 33 doc 파일 핵심 요약 |
| 33 | `33-changelog-history.md` | CHANGELOG History | 0.1.0 ~ 0.14.1 milestone + Removed |
| 34 | `34-tui-permissions-bridge.md` | TUI Views + Permissions + MCP Bridge body | Round 2 review gap-finder 발견 — Rust views + Python widgets/screens + claude/codex_permissions.py + MCPBridge body |
| 35 | `35-residual-modules.md` | Residual Modules — Round 3 gap fill | parallel_executor_models.py (AC outcome enum 본체) + execution_runtime_scope.py + verification/extractor.py + MCPClientManager |

## Topic 별 navigation

### "어떤 LLM 모델 / backend 가 어떻게 routing 되나?"
- 04 (PAL Router) — 3 tier escalation + downgrade Jaccard 0.80
- 11 (runtime abstraction) — 4 backend Protocol parity
- 29 (small modules) — Escalation + Downgrade body

### "어떻게 cancel 되나?"
- 28 (runner internals) — `cancellation_registry` + in-flight cancel
- 09 (evolutionary loop) — Recovery action set
- 10 (persistence) — EventStore = source of truth
- 20 (tests) — `test_inflight_cancellation.py` + `test_runner_cancellation.py` invariant

### "왜 EventStore 만 source of truth?"
- 10 (persistence) — SQLite schema + replay
- 33 (changelog) — StateStore/StateManager/RecoveryManager 4 클래스 제거 history (Plugin Phase 1 Unreleased)
- 28 (runner internals) — `runner._session_repo` alias

### "MCP tool 이 정확히 몇 개?"
- 12 (MCP hub) — 23 handler catalog (확정)
- 29 (small modules) — InputValidator + SecurityLayer + 4-tier defense

### "왜 ambiguity ≤ 0.2 + similarity ≥ 0.95?"
- 03 (Big Bang) — ambiguity 4-dimension weighted
- 25 (magic numbers) — 정확한 임계값
- 32 (docs deep-dive) — evolution-loop guide 의 "Two Mathematical Gates"
- 09 (evolutionary loop) — convergence

### "Hermes runtime 통합 PR 의 critical 결함?"
- 30 (external reviews) — C1 / H1-H5 / M1-M8 / R1-R5
- 31 (governance) — SECURITY response process

### "어떤 magic 숫자가 있나?"
- 25 (magic numbers) — 모든 임계값 + 한도
- 28 (runner internals) — `MAX_DEPTH=2` 정정 (docs error 였음)

### "agent persona 21 개가 어떻게 다른가?"
- 13 (plugin-skills-agents) — 21 persona body + JSON-only spec

### "skill 19 개가 정확히 뭐 하나?"
- 13 (plugin-skills-agents) — 19 skill 표 + 13 command stub
- 32 (docs deep-dive) — getting-started 의 onboarding flow

### "release 어떻게 build 되나?"
- 19 (CI/release) — release.yml + dev-publish.yml
- 33 (changelog) — version history

### "TUI 어떻게 구현됐나?"
- 15 (TUI implementations) — Python Textual + Rust SuperLightTUI 둘 다

## Phase 별 navigation (정정 — 실제 file 번호)

| Phase | Section |
|---|---|
| Phase 0 (Big Bang) | **03** + 25 (ambiguity) + 28 (init.py interview loop) |
| Phase 1 (PAL Router) | **04** + 11 + 29 (Escalation/Downgrade) |
| Phase 2 (Double Diamond) | **05** + 28 (parallel_executor) + 25 (MAX_DEPTH=2) |
| Phase 3 (Resilience) | **06** + 29 (Wonder/Reflect/Regression) |
| Phase 4 (Evaluation) | **07** + 29 (Verifier T1/T2 + ReDoS guard) + 20 (Stage 1 reuse) |
| Phase 5 (Secondary Loop) | **08** + 29 (BatchScheduler) |
| Evolutionary Loop | **09** + 28 (Recovery flow) |
| Persistence | **10** + 03 (EventStore) |

## File path 별 navigation (코드 location → section)

| 코드 path | Section |
|---|---|
| `src/ouroboros/orchestrator/runner.py` | 28 |
| `src/ouroboros/orchestrator/parallel_executor.py` | 28 + 05 |
| `src/ouroboros/cli/commands/init.py` | 28 + 14 |
| `src/ouroboros/evolution/{wonder,reflect,regression}.py` | 29 |
| `src/ouroboros/verification/verifier.py` | 29 + 07 |
| `src/ouroboros/secondary/scheduler.py` | 29 + 08 |
| `src/ouroboros/core/{git_workflow,file_lock,security}.py` | 29 |
| `src/ouroboros/mcp/{server,tools,client,bridge}/` | 12 + 29 (security) |
| `src/ouroboros/routing/{escalation,downgrade}.py` | 29 + 04 |
| `src/ouroboros/opencode/plugin/ouroboros-bridge.ts` | 16 |
| `src/ouroboros/agents/*.md` (21 persona) | 13 |
| `src/ouroboros/skills/*/SKILL.md` (19 skill) | 13 |
| `commands/*.md` (13 stub) | 13 |
| `tests/unit/orchestrator/test_*_cancellation.py` | 20 |
| `tests/e2e/test_session_persistence.py` | 20 + 10 |
| `.github/workflows/{release,dev-publish}.yml` | 19 |
| `CHANGELOG.md` | 33 |
| `CONTRIBUTING.md` | 31 |
| `SECURITY.md` / `HANDOFF.md` / `UNINSTALL.md` | 31 |
| `docs/getting-started.md` + `docs/architecture.md` | 32 |
| `docs/guides/*` (10 file) | 32 |
| `docs/runtime-guides/*` | 11 + 32 |
| `docs/api/{core,mcp,README}.md` | 32 |
| `docs/contributing/*` (5 file) | 32 |
| `docs/examples/workflows/*` | 32 |
| `Code-Review-{Claude,Codex}.md` | 30 |
| `examples/*.yaml` (4 seed) + `task_manager/*` | 32 (실 example) |
| `README.md` / `README.ko.md` | 32 |
| `crates/ouroboros-tui/` | 15 |
| `src/ouroboros/tui/` (Python Textual) | 15 |

## Confidence level (각 section 의 검증 정도)

| Section | Confidence | 근거 |
|---|---|---|
| 00-23 | ✅ High | Round 1 정독 + cross-ref 완료 |
| 24-27 | ✅ High | reference 자료 (deep-dive entrypoints / magic numbers / dependencies / gaps) |
| 28-33 | ✅ High | Ralph Round 1 에서 raw 코드 직접 정독 + cross-ref |
| 34 | ✅ High | Round 2 review gap-finder 결과 (TUI views + permissions + MCP bridge body) |
| 35 | ✅ High | Round 3 review gap fill (parallel_executor_models + execution_runtime_scope + extractor + MCPClientManager) |
| 25 (magic numbers) | ⚠️ Updated | MAX_DEPTH 5→2 정정 (docs/llms-full.txt 가 잘못된 정보 줬음) |
| 12 (MCP hub) | ⚠️ Updated | 21→23 tool 정정 (definitions.py 직접 읽음) |
| 16 (opencode bridge) | ⚠️ Updated | surfaceErr() doesn't exist — fail() + notify() 정정 |

## 미독 영역 (Section 27 이후 + Round 2 review 발견)

> Section 27 의 confirmation-gaps 가 이번 ralph round 의 정독 영역 빠져 있음 → 27 이 update 됨.
> Round 2 review (3 agent 병렬 verification) 가 추가 발견.

남은 미독 영역 (next ralph round 후보):

### Production code path-coverage 5.4% (gap-finder 발견)
- **Rust TUI views (6 file)** — `crates/ouroboros-tui/src/views/{dashboard,execution,lineage,logs,session_selector,mod}.rs` Section 15 cover 미흡
- **Python TUI widgets/screens (16 file)** — `tui/screens/{dashboard_v2,v3,confirm_rewind,lineage_detail,lineage_selector,hud_dashboard}.py` + 9 widget body 미분석
- **Permissions** — `claude_permissions.py` + `codex_permissions.py` 본문
- **MCP bridge body** — `mcp/bridge/bridge.py` + `mcp/tools/bridge_mixin.py` 본문
- **Orchestrator 21 잔여 file** — `heartbeat.py` / `execution_runtime_scope.py` / `opencode_event_normalizer.py` / `parallel_executor_models.py` 등
- **Router** — `router/dispatch.py` / `router/command_parser.py` 본문
- **Verification** — `verification/extractor.py` 본문

### 기타
- 개별 270+ 테스트 케이스 함수 본체 (현재는 4 invariant test 만 본체 분석)
- Codecov 실 % 결과
- Rust crate 의 cargo test
- Bun `ouroboros-bridge.test.ts`
- `playground/` example
- `docs/marketing/` / `screenshots/` / `videos/`

## 새 사용자 추천 path

1. **5분 onboarding**: 00 → 01
2. **개념 이해 (15분)**: 03 → 04 → 05 → 07 (Big Bang → PAL Router → Double Diamond → Evaluation)
3. **운영 가이드 (10분)**: 31 (governance) → 19 (CI) → 18 (config)
4. **확장 (필요 시)**: 12 (MCP) → 13 (skills/agents) → 11 (runtime backends)
5. **deep dive (advanced)**: 28 (runner internals) → 29 (small modules) → 30 (external reviews)
6. **history**: 33 (changelog) → 27 (gaps)

## 외부 자료 cross-ref

| 문서 | 본 분석에서 어디 |
|---|---|
| Q00/ouroboros GitHub | 모든 section 의 source |
| `docs/llms-full.txt` | 25 (정정 후) |
| `Code-Review-Claude.md` | 30 |
| `Code-Review-Codex.md` | 30 |
| README.md (영어) | 32 |
| README.ko.md | 32 |

## Round 2 review verification 결과

3 parallel review agent 가 33 section + INDEX 검증 (read-only):

| Reviewer | Verdict | 핵심 발견 |
|---|---|---|
| Cross-ref checker | needs major fix → **이 INDEX 가 fix 됨** | 9 section 라벨 mismatch, Phase 번호 1 칸 어긋남 (이 파일에서 정정) |
| Fact-checker | clean — minor errors only | Section 29 의 7 LOC ±1 drift (cosmetic, wc -l 방식 차이) |
| Gap-finder | path-level coverage 5.4% | 위 "미독 영역" 섹션에 통합. 추천 Section 34-39 = 다음 round |

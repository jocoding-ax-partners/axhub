# 25. Magic Numbers — Single Source of Truth

> 모든 핵심 상수의 단일 출처. 분석 검증 시 변경 추적용.

## Phase 0 — Big Bang

| 상수 | 값 | 위치 |
|---|---|---|
| Ambiguity 게이트 | ≤ 0.2 | `bigbang/ambiguity.py` `qualifies_for_seed_completion` |
| **Force-bypass score** | **0.19** | `cli/commands/init.py:FORCED_SCORE_VALUE` (`--force` 플래그가 강제로 0.19 주입 → 게이트 무조건 통과) |
| Goal/Constraint/Success 가중치 (greenfield) | 40 / 30 / 30 % | `bigbang/ambiguity.py` |
| Goal/Constraint/Success/Context 가중치 (brownfield) | 35 / 25 / 25 / 15 % | `bigbang/ambiguity.py` |
| LLM temperature (인터뷰) | 0.1 | `bigbang/interview.py` (재현성) |
| MAX_INITIAL_CONTEXT_LENGTH | 50,000 chars | `core/security.py` |
| MAX_USER_RESPONSE_LENGTH | 10,000 chars | `core/security.py` |
| MAX_SEED_FILE_SIZE | 1,000,000 bytes | `core/security.py` |
| MAX_LLM_RESPONSE_LENGTH | 100,000 chars | `core/security.py` |
| MIN_ROUNDS_BEFORE_EARLY_EXIT | 3 | `bigbang/interview.py` |
| SOFT_LIMIT_WARNING_THRESHOLD | 16 | `bigbang/interview.py` |
| MAX_INTERVIEW_ROUNDS | (제거됨, v0.3.0) | — |

## Phase 1 — PAL Router

| 상수 | 값 | 위치 |
|---|---|---|
| Tier 임계 (FRUGAL) | < 0.4 | `routing/tiers.py` |
| Tier 임계 (STANDARD) | < 0.7 | `routing/tiers.py` |
| Tier 임계 (FRONTIER) | ≥ 0.7 또는 critical | `routing/tiers.py` |
| Complexity 가중치 (tokens) | 30 % | `routing/complexity.py` |
| Complexity 가중치 (tools) | 30 % | `routing/complexity.py` |
| Complexity 가중치 (depth) | 40 % | `routing/complexity.py` |
| 정규화 임계 (tokens) | 4000 | `routing/complexity.py` |
| 정규화 임계 (tools) | 5 | `routing/complexity.py` |
| 정규화 임계 (depth) | 5 | `routing/complexity.py` |
| Tier 비용 배수 (FRUGAL/STANDARD/FRONTIER) | 1× / 10× / 30× | `routing/tiers.py` |
| Escalation 임계 | 2 연속 실패 | `routing/escalation.py` |
| Downgrade 임계 | 5 연속 성공 | `routing/downgrade.py` |
| Jaccard 유사 패턴 | ≥ 0.80 | `routing/router.py` |
| Cost 절감 주장 | 85 % (검증 못 함) | `skills/setup/SKILL.md` |

## Phase 2 — Double Diamond

| 상수 | 값 | 위치 |
|---|---|---|
| **MAX_DEPTH (recursion)** | **2** ⚠ | `parallel_executor.py:DEFAULT_MAX_DECOMPOSITION_DEPTH` (Section 28 정정 — 1차 라운드 5 는 docs 추정값) |
| **MAX_DECOMPOSITION_DEPTH** | **2** ⚠ | 같음. depth 0 → 1 → 2 강제 atomic |
| MIN_SUB_ACS | 2 | `parallel_executor.py:MIN_SUB_ACS` (분해 최소 child 수) |
| MAX_SUB_ACS | 5 | `parallel_executor.py:MAX_SUB_ACS` (분해 최대 child 수) |
| COMPRESSION_DEPTH | 3 (depth 3+ 에서 500 chars 절단) | `orchestrator/level_context.py` |
| Atomic 판정 | 1–2 파일 단일 초점 | `execution/atomicity.py` |
| MAX_RETRIES (Claude adapter) | 3 | `orchestrator/adapter.py:738` |
| RETRY_WAIT_INITIAL | 1.0 sec | `orchestrator/adapter.py:739` |
| RETRY_WAIT_MAX | 10.0 sec | `orchestrator/adapter.py:740` |
| Rate limit heartbeat | 5 sec | `orchestrator/rate_limit.py` (`RATE_LIMIT_HEARTBEAT_SECONDS`) |
| Rate limit max wait | RATE_LIMIT_MAX_WAIT_SECONDS | `orchestrator/rate_limit.py` |
| Default Anthropic RPM ceiling | DEFAULT_ANTHROPIC_RPM_CEILING | `orchestrator/rate_limit.py` |
| Default Anthropic TPM ceiling | DEFAULT_ANTHROPIC_TPM_CEILING | `orchestrator/rate_limit.py` |
| Parallel max concurrency | 10 (default) | config.yaml `execution.parallel_max_concurrency` |
| **STALL_TIMEOUT_SECONDS** | **300 sec (5 분)** | `parallel_executor.py:STALL_TIMEOUT_SECONDS` (무활동 → AC 포기) |
| **HEARTBEAT_INTERVAL_SECONDS** | **30 sec** | `parallel_executor.py:HEARTBEAT_INTERVAL_SECONDS` |
| **MAX_STALL_RETRIES** | **2** | `parallel_executor.py:MAX_STALL_RETRIES` |
| **DECOMPOSITION_TIMEOUT_SECONDS** | **60 sec** | `parallel_executor.py:DECOMPOSITION_TIMEOUT_SECONDS` |
| **_MIN_FREE_MEMORY_GB** | **2.0 GB** | `parallel_executor.py` (메모리 게이트) |
| **_MEMORY_CHECK_INTERVAL_SECONDS** | **5 sec** | `parallel_executor.py` |
| **_MEMORY_WAIT_MAX_SECONDS** | **120 sec** | `parallel_executor.py` |
| **_MAX_LEAF_RESULT_CHARS** | **1200** | `parallel_executor.py` (leaf evidence 절단) |
| **PROGRESS_EMIT_INTERVAL** | **10** | `runner.py` (msg 마다 progress event) |
| **SESSION_PROGRESS_PERSIST_INTERVAL** | **10** | `runner.py` (msg 마다 SQLite UPDATE) |
| **CANCELLATION_CHECK_INTERVAL** | **5** | `runner.py` (msg 마다 cancellation poll) |
| **Recovery snapshot unfinished cap** | **5 ACs** | `runner.py:_build_recovery_snapshot()` 의 `unfinished[:5]` |

## Phase 3 — Resilience

| 상수 | 값 | 위치 |
|---|---|---|
| SPINNING 임계 | 3 SHA-256 반복 | `resilience/stagnation.py` |
| OSCILLATION 임계 | 2 cycles | `resilience/stagnation.py` |
| NO_DRIFT 임계 | 3 회 | `resilience/stagnation.py` |
| NO_DRIFT epsilon | < 0.01 | `resilience/stagnation.py` |
| DIMINISHING_RETURNS 임계 | 3 회 | `resilience/stagnation.py` |
| DIMINISHING_RETURNS rate | < 0.01 | `resilience/stagnation.py` |
| Max lateral attempts | 5 (5 페르소나 한 번씩) | `resilience/lateral.py` |

## Phase 4 — Evaluation

| 상수 | 값 | 위치 |
|---|---|---|
| Coverage 임계 | ≥ 70 % | `evaluation/mechanical.py` |
| Semantic score 승인 | ≥ 0.8 | `evaluation/semantic.py`, `pipeline.py:243` |
| Semantic temperature | 0.2 | `evaluation/semantic.py` |
| Drift 트리거 | > 0.3 | `evaluation/trigger.py` |
| Uncertainty 트리거 | > 0.3 | `evaluation/trigger.py` |
| Stage 1 timeout | 600 sec (default) | `evaluation/mechanical.py` |
| Consensus majority | ≥ 2/3 (66 %) | `evaluation/consensus.py` |
| Consensus 모델 (default) | GPT-4o, Claude Sonnet 4, Gemini 2.5 Pro | `evaluation/consensus.py` |
| QA judge pass threshold | ≥ 0.80 | `agents/qa-judge.md` |
| QA judge revise threshold | ≥ 0.40 | `agents/qa-judge.md` |
| QA judge fail threshold | < 0.40 | `agents/qa-judge.md` |

## Evolution Loop

| 상수 | 값 | 위치 |
|---|---|---|
| Convergence similarity 임계 | ≥ 0.95 | `evolution/convergence.py` |
| Convergence 가중치 (name overlap) | 50 % | `evolution/convergence.py` |
| Convergence 가중치 (type match) | 30 % | `evolution/convergence.py` |
| Convergence 가중치 (exact match) | 20 % | `evolution/convergence.py` |
| Stagnation window | 3 세대 연속 ≥ 0.95 | `evolution/loop.py` |
| Oscillation period | period-2 (Gen N ≈ Gen N-2) | `evolution/loop.py` |
| Repetitive feedback 임계 | ≥ 70 % 질문 중복 / 3 세대 | `evolution/loop.py` |
| Hard cap generations | 30 | `evolution/loop.py` (`EvolutionaryLoopConfig.max_generations`) |
| Ralph max retries (lateral 후 evolve_step) | 2 | `scripts/ralph.py` |

## Drift Measurement

| 상수 | 값 | 위치 |
|---|---|---|
| Goal drift 가중치 | 50 % | `observability/drift.py` |
| Constraint drift 가중치 | 30 % | `observability/drift.py` |
| Ontology drift 가중치 | 20 % | `observability/drift.py` |
| Drift score 임계 | ≤ 0.3 | `observability/drift.py` |
| Retrospective 빈도 | 매 N 사이클 (default 5) | `observability/retrospective.py` |

## Persistence

| 상수 | 값 | 위치 |
|---|---|---|
| Append latency | < 10 ms p99 | `llms-full.txt:560` 측정값 |
| Query latency (1000 events) | < 50 ms | `llms-full.txt:561` 측정값 |
| Storage / event | ~1 KB | `llms-full.txt:562` 측정값 |
| Checkpoint interval | 5 분 | `persistence/checkpoint.py` |
| Rollback depth | 3-level | `persistence/checkpoint.py` |
| Checkpoint compression | 80 % reduction | `llms-full.txt:563` |
| Index 개수 | 5 (`aggregate_type`, `aggregate_id`, composite, `event_type`, `timestamp`) | `persistence/schema.py` |

## TUI

| 상수 | 값 | 위치 |
|---|---|---|
| Python TUI refresh rate | 500 ms | `tui/app.py` |
| Python TUI event processing | < 100 ms / update | `llms-full.txt:567` 측정값 |
| Rust TUI poll | 30 ticks (~3 s) | `crates/ouroboros-tui/src/main.rs:268` |
| Memory base | 50 MB | `llms-full.txt:570` |
| Memory per session | 10–100 MB | `llms-full.txt:571` |
| Agent pool concurrency | 2–10 | `llms-full.txt:574` |

## OpenCode Bridge (`ouroboros-bridge.ts`)

| 상수 | 값 | 위치 |
|---|---|---|
| MAX_BYTES (prompt cap) | 100,000 | `ouroboros-bridge.ts:21` |
| DEDUPE_MS (FNV-1a 윈도우) | 5,000 | `ouroboros-bridge.ts:22` |
| MAX_FANOUT (병렬 dispatch) | 10 | `ouroboros-bridge.ts:23` |
| MAX_SEEN (dedupe table) | 256 | `ouroboros-bridge.ts:24` |
| ID_LEN | 26 | `ouroboros-bridge.ts:25` |
| CHILD_TIMEOUT_MS (default) | 20 분 | `ouroboros-bridge.ts:28` |
| PATCH_RETRIES | 3 | `ouroboros-bridge.ts:29` |
| RESOLVE_RETRIES | 5 | `ouroboros-bridge.ts:30` |
| BACKOFF_MS | 100 | `ouroboros-bridge.ts:31` |
| Env override | OUROBOROS_CHILD_TIMEOUT_MS | — |

## Hooks / Scripts

| 상수 | 값 | 위치 |
|---|---|---|
| SessionStart hook timeout | 5 sec | `hooks/hooks.json` |
| UserPromptSubmit hook timeout | 5 sec | `hooks/hooks.json` |
| PostToolUse(Write\|Edit) hook timeout | 3 sec | `hooks/hooks.json` |
| Version check cache TTL | 86,400 sec (24 h) | `scripts/version-check.py:19` |
| Version check timeout | 5 sec (PyPI 호출) | `scripts/version-check.py:75` |
| Drift monitor active 윈도우 | 3,600 sec (1 hr) | `scripts/drift-monitor.py:42` |
| GitHub Releases curl timeout | 3 sec | `skills/interview/SKILL.md:33` |

## MCP

| 상수 | 값 | 위치 |
|---|---|---|
| MCP timeout (root .mcp.json) | 600 sec | `.mcp.json` |
| MCP timeout (plugin .mcp.json) | (default 30 sec) | `.claude-plugin/.mcp.json` |
| Job wait timeout (skill 권장) | 180 sec long-poll | `skills/run/SKILL.md` |
| Job wait timeout (ralph) | 120 sec | `skills/ralph/SKILL.md` |
| AC tree HUD max_nodes | 30 (default) | `mcp/tools/ac_tree_hud_handler.py` |

## CI

| 상수 | 값 | 위치 |
|---|---|---|
| Test job timeout | 15 분 | `.github/workflows/test.yml` |
| Python matrix | 3.12 / 3.13 / 3.14 | `.github/workflows/test.yml` |
| Release Python | 3.14 | `.github/workflows/release.yml` |
| Rust TUI binary 매트릭스 | 5 (linux x64/arm64, macos x64/arm64, windows x64) | `.github/workflows/release.yml` |
| Test coverage 정책 | ≥ 80 % | 글로벌 정책 (`common/testing.md`) |

## Limits / Caps Summary

| 도메인 | 임계 | 효과 |
|---|---|---|
| Ambiguity | ≤ 0.2 | Seed 생성 가능 |
| Convergence | ≥ 0.95 | 진화 정지 |
| Drift | > 0.3 | Stage 3 consensus 트리거 |
| Coverage | ≥ 0.7 | Stage 1 통과 |
| Semantic | ≥ 0.8 | Stage 2 승인 |
| Recursion depth | **2** ⚠ | hard limit (`DEFAULT_MAX_DECOMPOSITION_DEPTH`. 1차 라운드 5 정정) |
| Hard cap generations | 30 | 안전 밸브 |
| Prompt size | 100 KB (TS bridge) | truncation marker |

## Dynamic / Configurable

대부분 `config.yaml` 또는 환경 변수 override 가능. 특히:
- Ambiguity 가중치 (`clarification.weights`)
- Tier 비용 배수 (`economics.tiers`)
- Coverage 임계 (`evaluation.mechanical.coverage_threshold`)
- Convergence 임계 (`EvolutionaryLoopConfig.convergence_threshold`)
- Drift 가중치 (`drift.weights`)
- Anthropic 한계 (`OUROBOROS_ANTHROPIC_RPM_CEILING/TPM_CEILING`)

# 24. Deep-dive Entrypoints

> 추가 조사 시 즉시 진입할 수 있는 매핑. 사용자가 "X 를 더 자세히" 요청 시 이 표 사용.

## 핵심 메커니즘별 진입점

| 알고싶은 것 | 보아야 할 파일 + line/symbol | 추가 컨텍스트 |
|---|---|---|
| 멀티-AC stage1 reuse 정확한 메커니즘 | `evaluation/pipeline.py:113-184` | `tests/unit/evaluation/test_pipeline_stage1_reuse.py` |
| RuntimeHandle backend alias 추가 방법 | `orchestrator/adapter.py:223-280` (`_RUNTIME_HANDLE_BACKEND_ALIASES`, `_resolve_runtime_handle_backend`) | 추가 시 `runtime_factory.py` + `config/models.py` Literal 도 확장 |
| Cancellation registry 동시성 보장 | `orchestrator/runner.py:171-228` (`_cancellation_lock`, `request/clear/is_cancellation_requested`) | `tests/unit/orchestrator/test_runner_cancellation.py`, `test_inflight_cancellation.py` |
| MCP Job lifecycle | `mcp/job_manager.py`, `mcp/tools/job_handlers.py`, skill `run/SKILL.md:84-184` (low-token relay loop) | `view: "compact|summary|tree"` 옵션 |
| Lateral think multi-persona fan-out | CHANGELOG `[Unreleased]`, `mcp/tools/definitions.py` (`build_lateral_multi_subagent`), `opencode/plugin/ouroboros-bridge.ts` (MAX_FANOUT) | v0.30.0+ 신기능 |
| Brownfield mechanical.toml 자동 작성 | `skills/brownfield/SKILL.md` (`detect` 액션), `.ouroboros/mechanical.toml` 예시 | 단일 AI 호출로 작성 |
| Skill setup gate 우회 | `scripts/keyword-detector.py:21-26` (`SETUP_BYPASS_SKILLS`) | setup, help, qa 만 우회 |
| Stage 3 deliberative consensus | `evaluation/consensus.py` (`DeliberativeConsensus`, advocate/judge prompts), `strategies/devil_advocate.py`, `agents/{advocate,judge,consensus-reviewer}.md` | simple 모드와 다름 |
| RuntimeHandle persisted whitelist | `orchestrator/adapter.py:65-87` (`_OPENCODE_PERSISTED_METADATA_KEYS`) | OpenCode 만 적용 |
| 24h 버전 캐시 atomic write | `scripts/version-check.py:104-145` (tempfile.mkstemp + Path.replace) | race-free |
| TUI Python 이벤트 폴링 | `src/ouroboros/tui/app.py` + `events.py` (TUIState SSOT, 0.5s) | Textual 메시지 시스템 |
| TUI Rust 이벤트 폴링 | `crates/ouroboros-tui/src/main.rs:268-285` (poll_counter % 30 == 0) | 30 ticks ≈ 3s |
| Bridge plugin dispatch envelope | `src/ouroboros/opencode/plugin/ouroboros-bridge.ts` (`buildEnvelope`, `dupe`, `notify`, `fail`) | v22/v23 hardening |
| Interview Dialectic Rhythm Guard | `skills/interview/SKILL.md:238-251` | 비-사용자 답변 3 연속 → PATH 2 강제 |
| Seed-ready Acceptance Guard | `skills/interview/SKILL.md:218-230`, `agents/seed-closer.md` | main session 이 단일 게이트 keeper |
| Ambiguity 게이트 가중치 | `bigbang/ambiguity.py` (greenfield 40/30/30, brownfield 35/25/25/15) | docs `architecture.md:160` 일치 |
| Convergence 수식 분해 | `evolution/convergence.py` (0.5 name + 0.3 type + 0.2 exact) | 30 세대 hard cap |
| Stagnation 4 패턴 검출 | `resilience/stagnation.py` (`SpinningDetectedEvent` 등 4 이벤트) | stateless detection |
| 5 페르소나 dynamic loading | `resilience/lateral.py` (`_load_persona_strategies_from_md`) | `OUROBOROS_AGENTS_DIR` override |
| Recovery protocol prompt | `resilience/recovery.py` (`get_run_recovery_protocol_prompt`) | system prompt 자동 삽입 |
| MCP delegation hook (Claude) | `orchestrator/adapter.py:175-208` (`_build_delegated_tool_context_update`, `DELEGATED_*_ARG`) | 부모 Claude 세션 메타 자동 주입 |
| SharedRateLimitBucket | `orchestrator/rate_limit.py` (`acquire`, `force_reserve`, env override) | 5s heartbeat |
| Coordinator file conflict | `orchestrator/coordinator.py` (`FileConflict`, `CoordinatorReview`, `LevelCoordinator`) | 같은 레벨 형제 AC 충돌 시 review |
| EventStore `.mappings()` 패턴 | `project-context.md` Anti-Pattern #1 + `persistence/event_store.py` | `.scalars()` 사용 금지 |
| AgentMessage projection | `orchestrator/runtime_message_projection.py` (`message_tool_input`, `project_runtime_message`) | backend-neutral |
| Lifecycle state inference | `orchestrator/adapter.py:283-316` (`_runtime_handle_lifecycle_state`, `_RUNTIME_LIFECYCLE_STATE_BY_EVENT_TYPE`) | event type → state 매핑 |
| Tool Detail Extractor | `orchestrator/adapter.py:51-63` (`_TOOL_DETAIL_EXTRACTORS`) | 9 도구별 detail key |
| Transient Error 감지 | `orchestrator/adapter.py:743-754` (`TRANSIENT_ERROR_PATTERNS`, `_is_transient_error`) | 9 패턴 |
| AC Tree HUD 렌더링 | `mcp/tools/ac_tree_hud_render.py` + `ac_tree_hud_handler.py` | compact/summary/tree views |
| Drift 3-component 측정 | `observability/drift.py` (`calculate_goal_drift`, `_constraint_`, `_ontology_`) | 50/30/20 가중치 |
| Brownfield 영속화 | `persistence/brownfield.py` | scan + set_defaults |
| File lock heartbeat | `core/file_lock.py` + `core/worktree.py` (`heartbeat_lock`, `release_lock`) | stale lock 자동 해제 |

## Skill 별 진입점

| Skill | SKILL.md | 핵심 라인 |
|---|---|---|
| interview | `skills/interview/SKILL.md` | PATH 1a/1b/2/3/4 라인 117-198 |
| run | `skills/run/SKILL.md` | low-token relay loop 라인 95-185 |
| ralph | `skills/ralph/SKILL.md` | non-blocking poll 라인 47-99 |
| setup | `skills/setup/SKILL.md` | 6-step wizard 전체 |
| publish | `skills/publish/SKILL.md` | gh CLI Epic+Task 트리 |
| evolve | `skills/evolve/SKILL.md` | 진화 모니터 |
| brownfield | `skills/brownfield/SKILL.md` | scan + set_defaults + detect |
| pm | `skills/pm/SKILL.md` | PRD 생성 트랙 |
| qa | `skills/qa/SKILL.md` | qa-judge JSON 출력 |
| unstuck | `skills/unstuck/SKILL.md` | 5 페르소나 |

## 페르소나별 진입점

| 페르소나 | 파일 |
|---|---|
| Socratic Interviewer | `agents/socratic-interviewer.md` (CRITICAL ROLE BOUNDARIES, BREADTH CONTROL, STOP CONDITIONS) |
| Ontologist | `agents/ontologist.md` (4 fundamental questions: ESSENCE / ROOT CAUSE / PREREQUISITES / HIDDEN ASSUMPTIONS) |
| Seed Architect | `agents/seed-architect.md` (pipe-separated 출력 스키마) |
| Evaluator | `agents/evaluator.md` (3-stage 알고리즘) |
| QA Judge | `agents/qa-judge.md` (JSON-only, 5 dimensions, score thresholds 0.80/0.40) |
| Contrarian | `agents/contrarian.md` (5단계 도전) |
| Hacker | `agents/hacker.md` |
| Simplifier | `agents/simplifier.md` |
| Researcher | `agents/researcher.md` |
| Architect | `agents/architect.md` |
| Advocate | `agents/advocate.md` |
| Judge | `agents/judge.md` |
| Breadth Keeper | `agents/breadth-keeper.md` |
| Codebase Explorer | `agents/codebase-explorer.md` |
| Code Executor | `agents/code-executor.md` |
| Consensus Reviewer | `agents/consensus-reviewer.md` |
| Semantic Evaluator | `agents/semantic-evaluator.md` |
| Ontology Analyst | `agents/ontology-analyst.md` |
| Seed Closer | `agents/seed-closer.md` |
| Analysis Agent | `agents/analysis-agent.md` |
| Research Agent | `agents/research-agent.md` |

## 테스트별 진입점 (invariant 검증)

| 검증 | 테스트 파일 |
|---|---|
| Stage 1 AC-agnostic | `tests/unit/evaluation/test_pipeline_stage1_reuse.py` |
| 서브에이전트 격리 | `tests/unit/execution/test_subagent_isolation.py` |
| Codex 재귀 가드 | `tests/unit/orchestrator/test_codex_recursion_guard.py` |
| 실행 중 cancel | `tests/unit/orchestrator/test_inflight_cancellation.py` |
| Runner cancellation race | `tests/unit/orchestrator/test_runner_cancellation.py` |
| Atomicity 판정 | `tests/unit/orchestrator/test_parallel_executor_atomic_judgment.py` |
| MAX_DEPTH=2 enforce | `tests/unit/orchestrator/test_parallel_executor_recursive_depth.py` |
| 재시도 + resume | `tests/unit/orchestrator/test_parallel_executor_retry_resume.py` |
| InputValidator + 마스킹 | `tests/unit/core/test_security.py` |
| Ralph 정규식 파서 | `tests/unit/test_ralph_parser.py` |
| AC tree HUD 11 변형 | `tests/unit/mcp/tools/test_ac_tree_hud_*` (11 파일) |
| EventStore 영속성 | `tests/e2e/test_session_persistence.py` (678 LOC) |
| 전체 워크플로 | `tests/e2e/test_full_workflow.py` (496 LOC) |
| CLI 명령 | `tests/e2e/test_cli_commands.py` (431 LOC) |

## CHANGELOG 진입점

| 버전 | 주요 변경 |
|---|---|
| `[Unreleased]` | OpenCode subagent bridge plugin (200s → 10ms), Multi-persona fan-out, FREETEXT_FIELDS, v22/v23 hardening |
| `[0.14.1]` | Interview empty response 수정 + sub-agent turn exhaustion |
| `[0.13.4]` | EventStore 초기화 순서 |
| `[0.13.3]` | MCP 이중 등록 + isError 응답 + ValidationError 캐치 + nested string 검증 |
| `[0.13.2]` | rate_limit_event 미지원 메시지 + uvx --python 3.14 |
| `[0.3.0]` | MAX_INTERVIEW_ROUNDS 제거, tiered confirmation |
| `[0.2.0]` | security 모듈 (mask_api_key, InputValidator) |

## 다음 단계 (deep-dive 시작 추천)

가장 가치 있는 deep-dive 후보 (빈도 + 영향 기준):

1. **`evaluation/pipeline.py`** Stage 1 invariant 강화 가능성
2. **`orchestrator/parallel_executor.py`** 144K 분리 후보 식별
3. **`orchestrator/runner.py`** 109K 함수 그룹 매핑
4. **`opencode/plugin/ouroboros-bridge.ts`** v22/v23 hardening 검증
5. **`evolution/loop.py` + `convergence.py`** 자기참조 안전성 분석

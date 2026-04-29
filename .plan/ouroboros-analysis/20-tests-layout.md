# 20. Tests Layout

## 통계

- 총 270+ 테스트 파일
- 7797+ LOC (top-level unit/integration/e2e 합)
- `tests/` 가 `src/ouroboros/` 미러 구조

## 디렉토리 구조

```
tests/
├─ __init__.py
├─ conftest.py                                # 전역 fixture
├─ test-execution-plan.md                     # 테스트 계획 문서
├─ unit/                                       # ≥220 파일
├─ integration/                                # 8 파일
├─ e2e/                                        # 3 큰 파일 + conftest + config
└─ fixtures/                                   # 데이터
    ├─ router/skills/frontmatter-body/run/SKILL.md
    └─ test_atomicity_seed.yml
```

## Unit Tests (`tests/unit/`, ~220 파일)

### Top-level (16)

| 파일 | LOC | 역할 |
|---|---|---|
| `test_artifact_collector.py` | 115 | Stage 1 산출물 |
| `test_codex_artifacts.py` | 689 | Codex 산출물 |
| `test_convergence.py` | 636 | 진화 수렴 |
| `test_dashboard.py` | 323 | TUI 대시보드 |
| `test_dependencies_configured.py` | 87 | 의존성 검증 |
| `test_evolve_rewind.py` | 66 | 진화 되감기 |
| `test_evolve_step.py` | **1013** | 진화 단계 (가장 큰 unit) |
| `test_graceful_shutdown.py` | 495 | 정상 종료 |
| `test_main_entry_point.py` | 52 | CLI 진입 |
| `test_module_structure.py` | 39 | 패키지 구조 |
| `test_project_initialization.py` | 46 | 프로젝트 초기화 |
| `test_projector_rewind.py` | 471 | LineageProjector 되감기 |
| `test_ralph_parser.py` | 184 | ralph.py 정규식 파서 |
| `test_ralph_rewind.py` | 78 | ralph 되감기 |
| `test_regression.py` | 132 | 진화 회귀 감지 |
| `test_tooling_configuration.py` | 64 | 툴 설정 |
| `test_verification.py` | 454 | 검증 |

### 디렉토리별

```
unit/
├─ agents/test_loader.py
├─ bigbang/                              # 12 파일
│   ├─ test_ambiguity.py
│   ├─ test_brownfield.py
│   ├─ test_classifier_context.py
│   ├─ test_decide_later_items.py
│   ├─ test_explore.py
│   ├─ test_interview.py
│   ├─ test_interview_async_io.py
│   ├─ test_interview_no_web_search_hint.py
│   ├─ test_interview_research_prefix.py
│   ├─ test_pm_completion.py
│   ├─ test_pm_document_generator.py
│   ├─ test_pm_document_writer.py
│   ├─ test_pm_interview.py
│   ├─ test_pm_interview_brownfield_db.py
│   ├─ test_pm_seed_ac12.py
│   ├─ test_pm_seed_save.py
│   ├─ test_question_classifier.py
│   └─ test_seed_generator.py
├─ cli/                                  # 25 파일
│   ├─ commands/test_tui_command.py
│   ├─ formatters/                        # 6
│   │   ├─ test_console.py
│   │   ├─ test_panels.py
│   │   ├─ test_progress.py
│   │   ├─ test_prompting.py
│   │   ├─ test_tables.py
│   │   └─ test_workflow_display.py
│   ├─ test_bridge_plugin_hardening.py
│   ├─ test_bridge_plugin_lifecycle.py
│   ├─ test_cancel.py
│   ├─ test_config.py
│   ├─ test_doc_commands.py
│   ├─ test_init_pm_seed_detection.py
│   ├─ test_init_runtime.py
│   ├─ test_jsonc.py
│   ├─ test_main.py
│   ├─ test_mcp_doctor.py
│   ├─ test_mcp_nested_guard.py
│   ├─ test_mcp_shell_env.py
│   ├─ test_mcp_startup_cleanup.py
│   ├─ test_mcp_validate_transport_stderr.py
│   ├─ test_opencode_config.py
│   ├─ test_pm.py
│   ├─ test_pm_brownfield.py
│   ├─ test_pm_completion.py
│   ├─ test_pm_interactive_logging.py
│   ├─ test_pm_missing_litellm.py
│   ├─ test_pm_overwrite.py
│   ├─ test_pm_runtime_adapter.py
│   ├─ test_pm_select_repos.py
│   ├─ test_resume.py
│   ├─ test_run_qa.py
│   ├─ test_setup.py
│   └─ test_uninstall.py
├─ codex/test_cli_policy.py
├─ config/                                 # 3
│   ├─ test_loader.py
│   ├─ test_loader_env.py
│   └─ test_models.py
├─ core/                                   # 14
│   ├─ test_ac_tree.py
│   ├─ test_context.py
│   ├─ test_directive.py
│   ├─ test_errors.py
│   ├─ test_file_lock.py
│   ├─ test_git_workflow.py
│   ├─ test_initial_context.py
│   ├─ test_lineage.py
│   ├─ test_ontology_aspect.py
│   ├─ test_ontology_questions.py
│   ├─ test_retry.py
│   ├─ test_security.py
│   ├─ test_seed.py
│   ├─ test_ttl_cache.py
│   ├─ test_types.py
│   └─ test_worktree.py
├─ evaluation/                              # 14
│   ├─ test_artifact_collector_file_prefix.py
│   ├─ test_checklist.py
│   ├─ test_consensus.py
│   ├─ test_detector.py
│   ├─ test_json_utils.py
│   ├─ test_languages.py
│   ├─ test_mechanical.py
│   ├─ test_models.py
│   ├─ test_pipeline_stage1_reuse.py        # CRITICAL invariant 검증
│   ├─ test_pipeline_trigger_consensus.py
│   ├─ test_review_fixes.py
│   ├─ test_semantic.py
│   ├─ test_trigger.py
│   ├─ test_trigger_consensus.py
│   └─ test_verification_artifacts.py
├─ events/                                  # 3
│   ├─ test_base.py
│   ├─ test_control_events.py
│   └─ test_decomposition_events.py
├─ evolution/test_wonder_scope.py
├─ execution/                               # 4
│   ├─ test_atomicity.py
│   ├─ test_decomposition.py
│   ├─ test_double_diamond.py
│   └─ test_subagent_isolation.py
├─ hermes/test_artifacts.py
├─ mcp/                                     # ~30
│   ├─ bridge/                              # 5
│   │   ├─ test_bridge.py
│   │   ├─ test_config.py
│   │   ├─ test_factory.py
│   │   ├─ test_handler_wiring.py
│   │   └─ test_stability.py
│   ├─ client/                              # 3
│   │   ├─ test_adapter.py
│   │   ├─ test_adapter_transport_lifecycle.py
│   │   └─ test_manager.py
│   ├─ server/                              # 2
│   │   ├─ test_adapter.py
│   │   └─ test_security.py
│   ├─ test_errors.py
│   ├─ test_job_manager.py
│   ├─ test_types.py
│   └─ tools/                               # 15
│       ├─ test_ac_tree_hud_footer.py
│       ├─ test_ac_tree_hud_handler.py
│       ├─ test_ac_tree_hud_handler_completed.py
│       ├─ test_ac_tree_hud_handler_cursor_changed.py
│       ├─ test_ac_tree_hud_handler_invalid_session.py
│       ├─ test_ac_tree_hud_handler_no_execution.py
│       ├─ test_ac_tree_hud_handler_waiting.py
│       ├─ test_ac_tree_hud_max_nodes.py
│       ├─ test_ac_tree_hud_render_depth2.py
│       ├─ test_ac_tree_hud_render_depth3.py
│       ├─ test_ac_tree_hud_status_icons.py
│       ├─ test_ac_tree_hud_truncation.py
│       ├─ test_brownfield_handler.py
│       ├─ test_checklist_verify.py
│       ├─ test_definitions.py
│       ├─ test_evaluate_multi_ac.py
│       ├─ test_handler_subagent_wiring.py
│       ├─ test_interview_done_streak.py
│       ├─ test_lateral_think_handler.py
│       ├─ test_mcp_manager_wiring.py
│       ├─ test_pm_handler.py
│       ├─ test_pm_handler_pending_reframe.py
│       ├─ test_qa_parser.py
│       ├─ test_registry.py
│       ├─ test_round5_fixes.py
│       └─ test_subagent.py
├─ observability/                           # 3
│   ├─ test_drift.py
│   ├─ test_logging.py
│   └─ test_retrospective.py
├─ orchestrator/                            # 24 — 가장 많음
│   ├─ test_adapter.py
│   ├─ test_capabilities.py
│   ├─ test_codex_cli_runtime.py
│   ├─ test_codex_recursion_guard.py
│   ├─ test_command_dispatcher.py
│   ├─ test_control_plane.py
│   ├─ test_coordinator.py
│   ├─ test_dependency_analyzer.py
│   ├─ test_events.py
│   ├─ test_execution_runtime_scope.py
│   ├─ test_execution_strategy.py
│   ├─ test_hermes_runtime.py
│   ├─ test_inflight_cancellation.py
│   ├─ test_level_context.py
│   ├─ test_mcp_config.py
│   ├─ test_mcp_tools.py
│   ├─ test_opencode_runtime.py
│   ├─ test_parallel_executor.py
│   ├─ test_parallel_executor_atomic_judgment.py
│   ├─ test_parallel_executor_recursive_depth.py
│   ├─ test_parallel_executor_retry_resume.py
│   ├─ test_policy.py
│   ├─ test_rate_limit.py
│   ├─ test_runner.py
│   ├─ test_runner_cancellation.py
│   ├─ test_runtime_factory.py
│   ├─ test_runtime_message_projection.py
│   ├─ test_sandbox_class.py
│   ├─ test_session.py
│   └─ test_workflow_state.py
├─ persistence/                             # 5
│   ├─ test_brownfield_store.py
│   ├─ test_checkpoint.py
│   ├─ test_event_store.py
│   ├─ test_schema.py
│   └─ test_uow.py
├─ plugin/                                   # 3
│   ├─ agents/test_registry.py
│   ├─ orchestration/test_model_router.py
│   └─ skills/
│       ├─ test_keywords.py
│       └─ test_registry.py
├─ pm/test_renderer.py
├─ providers/                                # 7
│   ├─ test_base.py
│   ├─ test_claude_code_adapter.py
│   ├─ test_codex_cli_adapter.py
│   ├─ test_factory.py
│   ├─ test_gemini_cli_adapter.py
│   ├─ test_litellm_adapter.py
│   └─ test_opencode_adapter.py
├─ resilience/                               # 3
│   ├─ test_lateral.py
│   ├─ test_recovery.py
│   └─ test_stagnation.py
├─ router/                                    # 12
│   ├─ test_command_parser.py
│   ├─ test_dispatch.py
│   ├─ test_dispatch_result_shapes.py
│   ├─ test_extract_first_argument.py
│   ├─ test_invalid_dispatch_input_types.py
│   ├─ test_malformed_unknown_dispatch_errors.py
│   ├─ test_not_handled_resolution.py
│   ├─ test_registry.py
│   ├─ test_repository_dispatch_guard.py
│   ├─ test_router_resolution_pipeline.py
│   ├─ test_typed_frontmatter_fields.py
│   └─ test_valid_dispatch_normalization.py
├─ routing/                                  # 5
│   ├─ test_complexity.py
│   ├─ test_downgrade.py
│   ├─ test_escalation.py
│   ├─ test_router.py
│   └─ test_tiers.py
├─ scripts/                                   # 3
│   ├─ test_keyword_detector.py
│   ├─ test_session_start.py
│   └─ test_version_check.py
├─ secondary/                                # 2
│   ├─ test_scheduler.py
│   └─ test_todo_registry.py
├─ skills/test_skill_artifacts.py
└─ tui/                                      # 7
    ├─ test_app.py
    ├─ test_cancelled_display.py
    ├─ test_events.py
    ├─ test_lineage_viewer.py
    ├─ test_screens.py
    ├─ test_session_selector_replay.py
    └─ test_widgets.py
```

## Integration Tests (`tests/integration/`)

```
integration/
├─ conftest.py (179 LOC)
├─ mcp/                                       # 5
│   ├─ bridge/test_bridge_server_to_server.py
│   ├─ conftest.py
│   ├─ test_client_adapter.py
│   ├─ test_client_manager.py
│   └─ test_server_adapter.py
├─ plugin/test_orchestration.py
├─ test_cancel_subprocess_termination.py (102)
├─ test_codex_cli_passthrough_smoke.py (129)
├─ test_codex_skill_smoke.py (139)
├─ test_codex_skill_fallback.py (132)
└─ test_entry_point.py (30)
```

`test_cancel_subprocess_termination.py` — cancel → 자식 subprocess 정상 종료 검증.
`test_codex_*` — Codex CLI 통합 (smoke + skill + fallback).

## E2E Tests (`tests/e2e/`)

```
e2e/
├─ conftest.py (534 LOC)
├─ test_cli_commands.py (431)
├─ test_full_workflow.py (496)
├─ test_session_persistence.py (678)        # 가장 큰 e2e
└─ mcp_bridge_test_config.yaml
```

`test_full_workflow.py` — Phase 0 ~ Phase 5 + Evolution 전체 흐름.
`test_session_persistence.py` — EventStore 영속성 + replay + 재개.

## Fixtures (`tests/fixtures/`)

- `router/skills/frontmatter-body/run/SKILL.md` — frontmatter 파싱 테스트
- `test_atomicity_seed.yml` — atomicity 판정 테스트 시드

## 주요 검증 invariant

| 테스트 | 검증 |
|---|---|
| `test_pipeline_stage1_reuse.py` | Stage 1 AC-agnostic invariant (multi-AC dedup) |
| `test_subagent_isolation.py` | 서브에이전트 컨텍스트 격리 |
| `test_codex_recursion_guard.py` | Codex 재귀 호출 가드 |
| `test_inflight_cancellation.py` | 실행 중 cancel 안전성 |
| `test_runner_cancellation.py` | runner cancellation_registry race |
| `test_parallel_executor_atomic_judgment.py` | atomicity 판정 |
| `test_parallel_executor_recursive_depth.py` | MAX_DEPTH=2 enforce |
| `test_parallel_executor_retry_resume.py` | 재시도 + resume |
| `test_security.py` | InputValidator + 마스킹 |
| `test_repository_dispatch_guard.py` | repository dispatch 가드 |

## conftest 계층 구조

세 conftest 가 fixture 를 누적 (pytest 가 outermost → innermost 순으로 inherit):

### `tests/conftest.py` (57 LOC) — 전역

핵심 두 책임:

1. **CI 환경 의 Typer Rich console 우회**:
   ```python
   os.environ["_TYPER_FORCE_DISABLE_TERMINAL"] = "1"
   ```
   → CI 의 `GITHUB_ACTIONS` env 가 Typer 가 `force_terminal=True` 설정 → Rich 가 ANSI escape sequence 를 CliRunner string buffer 까지 inject (e.g. `--llm-backend` 의 hyphen 에 style sequence 삽입) → plain-text assertion 깨짐. `_TYPER_FORCE_DISABLE_TERMINAL=1` 가 Typer escape hatch.

2. **`close_test_owned_stores` autouse fixture**:
   ```python
   @pytest_asyncio.fixture(autouse=True)
   async def close_test_owned_stores(monkeypatch):
       # EventStore.__init__ + BrownfieldStore.__init__ monkeypatch
       # 으로 created instance 추적, 테스트 종료 시 reverse order 로 close()
   ```
   → aiosqlite leak warning 방지. `id()` 기반 dedup → 같은 store 두 번 close 안 함. close 가 awaitable 이면 `await`, 아니면 그냥 호출. close 실패는 `except Exception: pass`.

### `tests/integration/conftest.py` (179 LOC) — CLI 런타임 stub

5 클래스 + 2 fixture:

| 클래스 | 책임 |
|---|---|
| `FakeCLIStream` | async byte stream — `read()` 한 번 buffer 반환 후 빈 bytes |
| `FakeCLIStdin` | async stdin — `write()` 호출 기록, `drain()`/`close()`/`wait_closed()` no-op |
| `FakeCLIProcess` | subprocess double — stdout/stderr/stdin/returncode + `wait()`/`communicate()` |
| `RecordedCLICall` | frozen dataclass — `command`/`cwd`/`stdin_requested` 캡처 |
| `CLIScenario` | queued response — `final_message`/`stdout_events`/`stderr_text`/`returncode`. `stdout_text()` 가 events 를 NDJSON 으로 직렬화 |
| `OpenCodeSubprocessStub` | queue-backed stub — `queue()` 로 시나리오 등록, `__call__()` 으로 호출 시 pop |

→ `OpenCodeSubprocessStub.__call__` 가 command 의 `--output-last-message` flag index 검색 → 다음 인자를 path 로 해석 → 시나리오의 `final_message` 를 그 파일에 write. 이게 OpenCode CLI 의 실제 동작 시뮬.

`opencode_runtime_lifecycle_events` fixture 는 4 개 NDJSON event 표본:
- `thread.started` → `oc-session-123`
- `item.completed` (reasoning)
- `item.completed` (mcp_tool_call: `execute_seed`)
- `item.completed` (agent_message: 결과)

### `tests/e2e/conftest.py` (534 LOC) — 가장 큰 fixture 모음

22 fixture / 4 dataclass / 1 helper class:

**Seed fixtures**:
- `sample_seed` — TaskManager (4 AC, 2 ontology field, 2 evaluation principle, 1 exit condition)
- `minimal_seed` — Hello World (1 AC, ambiguity=0.1)
- `seed_yaml_content` — `yaml.dump(sample_seed.to_dict())`

**Temp dir fixtures**:
- `temp_dir`, `temp_state_dir`, `temp_seed_file`, `temp_db_path` (`sqlite+aiosqlite:///{tmp}/events.db`)

**Mock LLM provider** (`MockLLMProvider`):
- `responses` 시퀀스 + `_call_count` + `_call_history`
- `add_response()` chain
- `complete()` 가 `Result.ok(CompletionResponse)` 반환 — 시퀀스 끝나면 마지막 response 반복
- `mock_interview_llm_provider` — 5 typical interview 질문 pre-config

**Mock Claude Agent** (`MockClaudeAgentAdapter`):
- `runtime_backend` = "claude", `llm_backend` = "claude"
- `add_successful_execution(steps=N, final_message=...)` — assistant + tool (Read/Edit alternating) + result
- `add_failed_execution(error_message)` — assistant 1 + result with subtype=error
- `execute_task()` AsyncIterator — 시퀀스 idx clip 으로 다중 실행 지원

**Event store fixtures**:
- `event_store` async fixture — temp DB 초기화 + close
- `persisted_session` — 세션 + 2 progress entry pre-write

**CLI fixtures**:
- `cli_runner` — Typer `CliRunner`
- `mock_async_run` — `asyncio.run` patch context manager
- `WorkflowSimulator` helper — `configure_interview_flow(questions)` + `configure_successful_execution(steps)` + `create_seed_file(seed)`

## 핵심 invariant 테스트 본체 분석

### `test_pipeline_stage1_reuse.py` (191 LOC) — Issue #422 regression

> "These tests exercise the real `EvaluationPipeline.evaluate()` method — not handler-level mocks — to ensure future Stage 1 changes cannot break the shared-result invariant silently."

4 test 가 `stage1_result=` 파라미터 의 3-way semantics 검증:

| Test | stage1_result | 기대 동작 |
|---|---|---|
| `test_injected_passing_stage1_skips_mechanical_verify` | passing | `MechanicalVerifier.verify` NOT called + Stage 2 진행 + `final_approved=True` + `result.value.stage1_result is stage1` (id identity) |
| `test_no_stage1_result_calls_mechanical_verify` | None | `MechanicalVerifier.verify` MUST called |
| `test_injected_failing_stage1_causes_early_exit` | failing | `MechanicalVerifier.verify` NOT called + `SemanticEvaluator.evaluate` NOT called + early exit with `final_approved=False` |
| `test_injected_passing_stage1_allows_stage2` | passing | `SemanticEvaluator.evaluate` MUST called |

→ multi-AC 평가 시 첫 AC 의 Stage 1 결과를 **재사용** (AC-agnostic). 이게 깨지면 N AC 마다 Stage 1 N 번 실행 = N 배 느림.

**id identity assertion**:
```python
assert result.value.stage1_result is stage1   # is — 같은 객체
```
→ 새 객체로 wrap 하지 않고 **같은 인스턴스** 통과 검증. 깊은 복사가 일어나면 fail.

### `test_inflight_cancellation.py` (913 LOC) — runner.cancellation flow

핵심 검증:
- `is_cancellation_requested("sess_1")` 이 cancel 등록 후 True 반환
- `get_pending_cancellations()` 가 `frozenset({"sess_1"})` 반환 (mutable set 아님 — 외부 변경 차단)
- `result.value["status"] == "cancellation_requested"`
- `result.value["in_flight"] is True` (실행 중 cancel)
- `result.value["reason"] == "Test"` / `"Timed out after 30m"`
- 멱등성 — 두 번 cancel 호출해도 둘 다 `is_ok` (idempotent)
- 정리 — cancel 후 `runner.active_sessions` 에서 제거
- 통계 — `result.value.messages_processed == 42` (cancel 시점 message count 보존), `result.value.success is False`, `result.value.duration_seconds >= 0`
- 등록 안 된 execution 의 cancel 은 `result.is_ok` (graceful no-op)

### `test_runner_cancellation.py` (684 LOC) — cancellation registry semantics

핵심 검증:
- `is_cancellation_requested("sess_999")` False (등록 안 한 session)
- `get_pending_cancellations()` 가 unmodifiable `frozenset` 반환 (`isinstance(pending, frozenset)`)
- 빈 상태 = `frozenset()` (None 이나 set() 아님)
- `runner.active_sessions["exec_1"] == "sess_1"` 매핑 정합성
- `runner.session_repo is runner._session_repo` (private alias 노출 검증)
- `CancellationError` 의 `session_id`/`reason`/`__str__` 형식

### `test_session_persistence.py` (678 LOC) — EventStore replay invariants

핵심 검증:
- 세션 시작 → `orchestrator.session.started` event with `seed_id` + `execution_id`
- 두 세션은 다른 `session_id` 받음
- 사용자 지정 `execution_id` 통과 (auto-generate 아님)
- 정상 종료 → `orchestrator.session.completed` event
- 실패 → `orchestrator.session.failed` event with `result.is_ok` (Result envelope 자체는 ok, 안의 `success` 가 False)
- 세션 재구성 → `tracker.session_id` / `tracker.execution_id` / `tracker.status == SessionStatus.RUNNING` 정확 복원

→ EventStore = source of truth. session 객체 자체는 derived state.

## Codecov

`test.yml` 가 매 매트릭스 (3.12/3.13/3.14) 별도 flag 로 업로드:
```yaml
flags: unittests
name: codecov-${{ matrix.python-version }}
```

`fail_ci_if_error: false` — Codecov 실패 시 CI 통과 (커버리지 게이트 아님).

## 80% Coverage 정책

글로벌 `~/.claude/rules/common/testing.md` 가 80% 강제. `pyproject.toml` 에 명시 임계 없음 — 정책만.

## 검증 안 한 것

- 개별 테스트 케이스 (각 `test_*.py` 안의 함수)
- Codecov 실 커버리지 % 
- Rust crate 의 `cargo test`
- Bun 테스트 (`ouroboros-bridge.test.ts`)

# 28. Deep Runner Internals — `runner.py` + `parallel_executor.py` + `init.py`

> Round 2 deep-dive. 처음 라운드에서 grep + 첫 300 LOC 만 본 거대 파일 본체 전부 정독. **Section 25 magic numbers 다수 정정** 포함.

## 28.1 `orchestrator/runner.py` (2722 LOC) 본체

### Class 구조

```python
class OrchestratorRunner:
    """단일 세션 메시지 루프 + 병렬 위임 + 재개"""

    PROGRESS_EMIT_INTERVAL = 10           # 메시지마다 progress event
    SESSION_PROGRESS_PERSIST_INTERVAL = 10 # 메시지마다 SQLite UPDATE
    CANCELLATION_CHECK_INTERVAL = 5       # 메시지마다 cancellation 폴
```

### 핵심 메서드 (라인 위치 + 책임)

| 메서드 | 책임 |
|---|---|
| `_register_session(exec_id, session_id)` | in-memory 세션 캐시 + cancellation 가능 표시 |
| `_unregister_session(exec_id, session_id)` | 위 캐시 삭제 + ACTIVE_SESSIONS 정리 |
| `_cleanup_pre_execution_state(exec_id, session_id, *, session_registered)` | failure 경로에서 락/캐시 모두 release |
| `_deserialize_runtime_handle(progress: dict)` | tracker.progress dict → `RuntimeHandle` 복원 (resume 시) |
| `_seed_runtime_handle(handle, *, tool_catalog)` | 도구 카탈로그를 metadata 에 주입 + control_plane state 직렬화 |
| `_get_merged_tools(session_id, tool_prefix)` | DEFAULT_TOOLS + MCP 도구 병합 + policy evaluator (`_evaluate_tool_catalog_policy`) + capability event 발행 |
| `_check_cancellation(session_id)` | `is_cancellation_requested()` async 호출 |
| `_handle_cancellation(...)` | cancellation event 발행 + session_repo.mark_cancelled + 락 release |
| `execute_seed(seed)` | 1차 진입점 — prepare_session → execute_precreated_session 위임 |
| `prepare_session(seed)` | 세션 trackier 생성 + session_started event |
| `execute_precreated_session(seed, tracker)` | 메시지 루프 + recovery + parallel-mode 분기 |
| `_execute_parallel(seed, exec_id, tracker, ...)` | DependencyAnalyzer 호출 → ParallelACExecutor 위임 |
| `resume_session(session_id, seed)` | 별도 메서드. session_repo.reconstruct_session() → terminal 상태면 거절 → 메시지 루프 재개. 결과는 `mark_completed` / `mark_paused` (recoverable) / `mark_failed` |

### 메시지 루프 (line 1700–1880, async closure `_consume_task_stream`)

```python
async def _consume_task_stream(*, prompt, resume_handle, status):
    nonlocal messages_processed, success, final_message, cancelled_result

    async with aclosing(self._adapter.execute_task(...)) as message_stream:
        async for message in message_stream:
            messages_processed += 1
            projected = project_runtime_message(message)

            if messages_processed % CANCELLATION_CHECK_INTERVAL == 0:
                if await self._check_cancellation(...):
                    cancelled_result = await self._handle_cancellation(...)
                    break

            tracker = await self._update_and_persist_progress(...)
            state_tracker.process_runtime_message(message)

            # Console log
            if projected.tool_name and projected.tool_name != last_tool:
                console.print(f"  🔧 {projected.tool_name}")
            elif projected.message_type == "assistant" and projected.content:
                console.print(f"  💭 {content}")

            if current_completed > last_completed_count:
                console.print(f"  ✓ AC {current_completed} completed")

            # Emit workflow_progress event (for TUI)
            workflow_event = create_workflow_progress_event(...)
            await self._event_store.append(workflow_event)

            # 매 PROGRESS_EMIT_INTERVAL=10 마다 drift 측정
            if messages_processed % PROGRESS_EMIT_INTERVAL == 0:
                drift_metrics = DriftMeasurement().measure(...)
                await self._event_store.append(create_drift_measured_event(...))

            if message.is_final:
                final_message = message.content
                success = not message.is_error
```

### Recovery 분기 (line 1888–1920)

```python
if cancelled_result is None and not success and runtime_handle is not None:
    planner = RecoveryPlanner()
    recovery_action = planner.plan(_build_recovery_snapshot())
    if (recovery_action.kind == RecoveryActionKind.INJECT_LATERAL_DIRECTIVE
        and recovery_action.directive
        and recovery_action.persona is not None):
        recovery_interventions_used += 1
        recovery_personas.append(recovery_action.persona.value)
        await self._event_store.append(create_recovery_applied_event(...))
        # 같은 runtime_handle 재사용해서 lateral directive 주입
        runtime_handle = await _consume_task_stream(
            prompt=recovery_action.directive,
            resume_handle=runtime_handle,
            status=status,
        )
```

→ Section 9 (resilience) 가 1차 round 에서 추정한 "stagnation 감지 → lateral persona 주입" 흐름 = `RecoveryPlanner.plan()` 직접 호출. 페르소나는 5 페르소나 한 번씩 (max_lateral_attempts=5).

### `_build_recovery_snapshot()` 로컬 클로저

```python
def _build_recovery_snapshot() -> RecoverySnapshot:
    unfinished = [
        f"{ac.index}. {ac.content}"
        for ac in state_tracker.state.acceptance_criteria
        if ac.status.value != "completed"
    ]
    unfinished_text = "\n".join(unfinished[:5]) or "None"  # top 5
    problem_context = f"Goal: {seed.goal}\nUnfinished AC:\n{unfinished_text}\n\nPrevious final message:\n{final_message[:1000]}"
    current_approach = "..."  # standard prefix
    return RecoverySnapshot(
        problem_context=...,
        current_approach=...,
        messages_processed=messages_processed,
        completed_count=...,
        total_count=...,
        final_error=final_message,
        used_personas=tuple(ThinkingPersona(p) for p in recovery_personas),
        interventions_used=recovery_interventions_used,
    )
```

→ unfinished AC 의 첫 5 개만 RecoveryPlanner 로 전달. 5+ 미완료 AC 가 있으면 본문에서 truncate. **이 5 cap 은 docs 미공개**.

### Cancellation 처리

```python
except asyncio.CancelledError:
    if await is_cancellation_requested(tracker.session_id):
        return await self._handle_cancellation(...)  # graceful event 발행
    self._unregister_session(...)
    raise  # 재발사
```

→ asyncio.CancelledError 도 re-raise 가 default. cancellation 이 explicit 이지 않으면 outer task group 으로 propagate.

### Terminal event 발행 (TUI 단일 스트림 detection)

```python
terminal_event = create_execution_terminal_event(
    execution_id=exec_id,
    session_id=tracker.session_id,
    status="completed" if success else "failed",
    summary=completion_summary if success else None,
    error_message=final_message if not success else None,
    messages_processed=messages_processed,
)
await self._event_store.append(terminal_event)
```

→ session aggregate 외에도 execution event stream 에 mirror — TUI 가 polling 안 하고 단일 stream 으로 detect 가능.

### `_execute_parallel` (line 2083–2335)

```python
async def _execute_parallel(self, seed, exec_id, tracker, merged_tools,
                              tool_catalog, system_prompt, start_time,
                              externally_satisfied_acs):
    analyzer = self._build_dependency_analyzer()
    dep_result = await analyzer.analyze(seed.acceptance_criteria)

    if dep_result.is_err:
        # Fallback: 모든 AC 하나의 level 에 (단순 병렬)
        all_indices = tuple(range(len(seed.acceptance_criteria)))
        dependency_graph = DependencyGraph(...)
    else:
        dependency_graph = dep_result.value

    execution_plan = dependency_graph.to_execution_plan()
    parallel_executor = ParallelACExecutor(
        adapter=self._adapter,
        event_store=self._event_store,
        console=self._console,
        enable_decomposition=self._enable_decomposition,
        max_concurrent=self._max_parallel_workers,
        max_decomposition_depth=self._max_decomposition_depth,
        inherited_runtime_handle=self._inherited_runtime_handle,
        task_cwd=self._effective_cwd(),
        checkpoint_store=self._checkpoint_store,
    )

    # cancellation 먼저 체크
    if await self._check_cancellation(...):
        return await self._handle_cancellation(...)

    parallel_result = await parallel_executor.execute_parallel(...)

    # 실행 후도 cancellation 체크 (mid-flight 취소된 경우)
    if await self._check_cancellation(...):
        return await self._handle_cancellation(...)
```

→ **DependencyAnalyzer 실패 시 fallback** = 모든 AC 단일 level 병렬. 이 fallback 은 docs/시각자료 누락 영역.

### `resume_session` (line 2337–2708)

```python
async def resume_session(self, session_id: str, seed: Seed):
    session_result = await self._session_repo.reconstruct_session(session_id)
    tracker = session_result.value

    if tracker.status in (COMPLETED, CANCELLED, FAILED):
        return Result.err(...)  # terminal 상태 거절

    # Build resume prompt
    resume_prompt = f"""Continue executing the task from where you left off.

{build_task_prompt(seed)}

Note: This is a resumed session. Please continue from where execution was interrupted.
"""
    # 메시지 루프 (execute 와 거의 동일)
    # ...

    if success:
        await session_repo.mark_completed(...)
    elif recoverable_resume_failure:
        await session_repo.mark_paused(session_id, reason=..., resume_hint="Retry the same --resume session after fixing the runtime/tooling issue.")
    else:
        await session_repo.mark_failed(...)
```

→ recoverable failure 는 PAUSED 로 mark — 다음 resume 시 재시도 가능. **이 3-way 분기는 Section 5 에 누락된 detail**.

## 28.2 `orchestrator/parallel_executor.py` (3479 LOC) 본체

### Module 상수 (Section 25 정정 대상)

```python
DEFAULT_MAX_DECOMPOSITION_DEPTH = 2     # ← Section 25 의 5 와 다름
MIN_SUB_ACS = 2
MAX_SUB_ACS = 5
DECOMPOSITION_TIMEOUT_SECONDS = 60.0
STALL_TIMEOUT_SECONDS = 300.0           # 5분 무활동 → 포기
HEARTBEAT_INTERVAL_SECONDS = 30.0
MAX_STALL_RETRIES = 2

_MIN_FREE_MEMORY_GB = 2.0               # 메모리 게이트
_MEMORY_CHECK_INTERVAL_SECONDS = 5.0
_MEMORY_WAIT_MAX_SECONDS = 120.0
_MAX_LEAF_RESULT_CHARS = 1200           # leaf evidence 절단

_STALL_SENTINEL = "__OUROBOROS_STALL__"
_IMPLEMENTATION_SESSION_KIND = "implementation"

_AC_RUNTIME_SCOPE_METADATA_KEYS = (
    "session_scope_id", "ac_index", "parent_ac_index", "sub_ac_index",
)
_AC_RUNTIME_OWNERSHIP_METADATA_KEYS = (
    "ac_id", "scope", "session_role", "session_scope_id",
    "ac_index", "parent_ac_index", "sub_ac_index",
)
_AC_RUNTIME_RESUME_METADATA_KEYS = (
    "session_id", "server_session_id", "resume_session_id",
    "native_session_id", "transcript_path",
)
_REUSABLE_RUNTIME_EVENT_TYPES = {
    "session.started", "thread.started", "session.resumed", "thread.resumed",
}
_NON_REUSABLE_RUNTIME_EVENT_TYPES = {
    "session.completed", "session.failed", "session.cancelled",
    "thread.completed", "thread.failed", "thread.cancelled",
}
```

### Recursive AC execution flow (`_execute_single_ac`)

```python
async def _execute_single_ac(self, ac_index, ac_content, ...,
                               depth=0, ...):
    # 1. Decompose 시도 (depth < max_decomposition_depth 일 때만)
    if self._enable_decomposition and depth < self._max_decomposition_depth:
        sub_acs = await self._try_decompose_ac(...)
        if sub_acs and len(sub_acs) >= MIN_SUB_ACS:
            # ▶ 재귀: sub_acs 를 _execute_single_ac 로 다시 호출
            for idx, sub_ac in enumerate(sub_acs):
                sub_result = await self._execute_single_ac(
                    ac_index=ac_index,  # 부모 ac_index 유지
                    ac_content=sub_ac,
                    depth=depth + 1,
                    is_sub_ac=True,
                    parent_ac_index=ac_index,
                    sub_ac_index=idx,
                    ...
                )
            # leaf result 결합 → composite ACExecutionResult 반환

    # 2. Atomic 실행
    return await self._run_atomic_ac(...)
```

→ **재귀 깊이는 depth=0 (top-level) → depth=1 (sub) → depth=2 면 강제 atomic**. `MAX_DECOMPOSITION_DEPTH=2` 라 실제로 깊은 트리는 안 만듦. 깊은 분해를 원하면 ParallelACExecutor 생성 시 `max_decomposition_depth` 인자 override 필요.

### Stage 실행 (`execute_parallel` 메인 루프)

```python
async with anyio.create_task_group() as outer_tg:
    outer_tg.start_soon(self._resilient_progress_emitter, ...)  # 백그라운드

    for stage in execution_plan.stages:
        level_idx = stage.index
        level = self._get_stage_ac_indices(stage)
        stage_batches = self._get_stage_batches(stage)

        # RC3: 체크포인트 복구로 이미 끝난 level skip
        if level_idx < resume_from_level:
            continue

        # 의존성 검증
        for ac_idx in level:
            deps = execution_plan.get_dependencies(ac_idx)
            if any(dep in failed_indices or dep in blocked_indices for dep in deps):
                blocked.append(ac_idx)
            elif ac_idx in external_completed:
                externally_satisfied.append(ac_idx)
            else:
                executable.append(ac_idx)

        # Batch 단위 병렬 실행
        for batch in stage_batches:
            batch_results = await self._execute_ac_batch(...)
            for ac_idx, result in zip(batch, batch_results):
                if isinstance(result, BaseException):
                    failed_indices.add(ac_idx)
                elif result.error == _STALL_SENTINEL:
                    # STALL_TIMEOUT_SECONDS=300 후 포기
                    failed_indices.add(ac_idx)
                else:
                    if result.success:
                        completed_count += 1
                    elif result.is_blocked:
                        blocked_indices.add(ac_idx)
                    else:
                        failed_indices.add(ac_idx)

        # Coordinator: 파일 충돌 감지 + 리뷰 (Approach A)
        conflicts = self._coordinator.detect_file_conflicts(level_ac_results)
        if conflicts:
            review = await self._coordinator.run_review(...)
            level_ctx = LevelContext(..., coordinator_review=review)
            stage_result = replace(stage_result, coordinator_review=review)

        # RC3: level 끝마다 체크포인트 저장
        if self._checkpoint_store:
            checkpoint = CheckpointData.create(
                seed_id=seed_id,
                phase="parallel_execution",
                state={
                    "session_id": ...,
                    "execution_id": ...,
                    "completed_levels": level_idx + 1,
                    "ac_statuses": {str(k): v for k, v in ac_statuses.items()},
                    "failed_indices": sorted(failed_indices),
                    "completed_count": completed_count,
                    "level_contexts": serialize_level_contexts(level_contexts),
                },
            )
            self._checkpoint_store.save(checkpoint)

    outer_tg.cancel_scope.cancel()  # 백그라운드 emitter 종료
```

### AC outcome 5-way 분류

```python
class ACExecutionOutcome(StrEnum):
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    BLOCKED = "blocked"               # 의존성 실패로 skip
    INVALID = "invalid"               # dependency graph 누락
    SATISFIED_EXTERNALLY = "satisfied_externally"  # --skip-completed
```

→ Section 5 의 "병렬 실행 outcome" 부분에 5-way 분류 추가 필요. 1차 라운드는 success/failure 만 다룸.

### Runtime handle scope normalization (line 526–710)

각 AC 가 자기 scope 의 RuntimeHandle 만 reuse 하도록 강제. 다른 AC scope 의 native_session_id 가 흘러 들어오면 `scrub_resume_state=True` 로 metadata 만 남기고 resume state 는 폐기.

```python
def _normalize_ac_runtime_handle(self, runtime_handle, *, runtime_scope,
                                   ac_index, is_sub_ac, parent_ac_index,
                                   sub_ac_index, retry_attempt, source,
                                   require_resume_scope_match):
    expected_metadata = self._build_expected_ac_runtime_metadata(...)

    if require_resume_scope_match and self._is_resumable_runtime_handle(...):
        if not self._runtime_handle_matches_ac_scope_for_resume(...):
            log.warning("parallel_executor.ac.runtime_handle_scope_rejected", ...)
            return None  # 다른 scope 의 resume handle 거절

    scrub_resume_state = self._runtime_handle_claims_foreign_ac_scope(...)
    if scrub_resume_state:
        log.warning("parallel_executor.ac.runtime_handle_scope_scrubbed", ...)

    return self._bind_runtime_handle_to_ac_scope(
        runtime_handle, expected_metadata=expected_metadata,
        scrub_resume_state=scrub_resume_state,
    )
```

→ **AC scope 격리 보장**. AC1 의 Claude 세션이 AC2 의 retry 에서 실수로 resume 되는 일을 방지. 이 보호장치는 docs/architecture.md 누락된 detail.

### Recovery discontinuity tracking (line 1118–1257)

같은 AC 내에서 backend session 이 새로 시작되면 (ex. Claude 가 401 후 재인증으로 native_session_id 바뀜):

```python
@classmethod
def _build_recovery_discontinuity(cls, *, previous_handle, current_handle,
                                    runtime_identity):
    if previous_handle is None or previous_handle.resume_session_id is None:
        return None
    if cls._runtime_handle_same_session(previous_handle, current_handle):
        return None  # 같은 세션 → discontinuity 아님

    current_event_type = current_handle.metadata.get("runtime_event_type")
    replacement_event = isinstance(current_event_type, str) and \
        current_event_type.strip().lower() in {"session.started", "thread.started"}
    # native/server session_id 가 바뀐지도 검증

    return {
        "reason": "replacement_session",
        "failed": {
            "session_id": previous_native,
            "server_session_id": previous_server,
            "resume_session_id": previous_handle.resume_session_id,
            "turn_id": cls._runtime_turn_id(...),
            "turn_number": failed_turn_number,
        },
        "replacement": {
            "session_id": current_native,
            "server_session_id": current_server,
            "resume_session_id": current_handle.resume_session_id,
            "turn_id": cls._default_turn_id(...),
            "turn_number": replacement_turn_number,
        },
    }
```

→ event 에 `recovery_discontinuity` 필드 첨부. TUI / replay 가 turn 끊김을 식별.

## 28.3 `cli/commands/init.py` (855 LOC) 본체

### `_DefaultStartGroup` shorthand

```python
class _DefaultStartGroup(typer.core.TyperGroup):
    """`ouroboros init "..."` 형태로 첫 인자가 알려진 sub-command 가 아니면 자동으로 `start` 사용."""

    def get_command(self, ctx, cmd_name):
        cmd = super().get_command(ctx, cmd_name)
        if cmd is not None:
            return cmd
        # cmd_name 이 quoted prompt → start 로 forward
        if cmd_name and not cmd_name.startswith("-"):
            return super().get_command(ctx, "start")
        return None
```

→ `ouroboros init "Build me a CLI"` 가 `ouroboros init start "Build me a CLI"` 와 동치. **이 shorthand 는 README 에 명시 안 됨**.

### Enum 정의

```python
class SeedGenerationResult(StrEnum):
    SUCCESS = "success"
    CANCELLED = "cancelled"
    CONTINUE_INTERVIEW = "continue_interview"  # 사용자가 다시 인터뷰

class AgentRuntimeBackend(StrEnum):
    CLAUDE = "claude"
    CODEX = "codex"
    OPENCODE = "opencode"
    HERMES = "hermes"

class LLMBackend(StrEnum):
    CLAUDE_CODE = "claude_code"
    LITELLM = "litellm"
    CODEX = "codex"
    OPENCODE = "opencode"
```

### Force-bypass 상수

```python
FORCED_SCORE_VALUE = 0.19  # ambiguity ≤ 0.2 강제 통과
```

→ `--force` 플래그 시 ambiguity 점수를 0.19 로 강제 → seed 생성 게이트 무조건 통과. **이 backdoor 는 docs 미공개**.

### PM seed 자동 검출 + 선택 흐름

```python
def _has_dev_seed(seeds_dir: Path) -> bool:
    if (seeds_dir / "seed.json").exists():
        return True
    return any(not yaml.name.startswith("pm_seed_")
               for yaml in seeds_dir.glob("*.yaml"))

def _find_pm_seeds(seeds_dir: Path) -> list[Path]:
    return sorted(seeds_dir.glob("pm_seed_*.yaml"))

def _prompt_pm_seed_selection(pm_seeds: list[Path]) -> Path | None:
    _notify_pm_seed_detected(pm_seeds)  # 박스 그래픽 알림
    if len(pm_seeds) == 1:
        # yes/no
        return pm_seeds[0] if Confirm.ask(...) else None
    # multiple → 번호 선택 + 0=skip
    choice = Prompt.ask(...)
    return pm_seeds[int(choice) - 1] if int(choice) > 0 else None
```

→ Dev interview 전에 `~/.ouroboros/seeds/pm_seed_*.yaml` 자동 스캔. PM seed 가 발견되고 dev seed 가 없으면 user 에게 선택 prompt. 선택된 PM seed 의 YAML 본문이 dev interview 의 `initial_context` 로 사용됨.

### `start` command 실행 분기

```python
if not resume:
    seeds_dir = Path.home() / ".ouroboros" / "seeds"
    if not _has_dev_seed(seeds_dir):
        pm_seeds = _find_pm_seeds(seeds_dir)
        if pm_seeds:
            if context:
                _notify_pm_seed_detected(pm_seeds)
                use_pm = Confirm.ask(...)
                if use_pm:
                    selected = _prompt_pm_seed_selection(pm_seeds) if len(pm_seeds) > 1 else pm_seeds[0]
                    if selected:
                        context = _load_pm_seed_as_context(selected)
            else:
                # context 없을 때 PM seed 가 1차 옵션
                selected = _prompt_pm_seed_selection(pm_seeds)
                if selected:
                    context = _load_pm_seed_as_context(selected)

    if not context:
        # Welcome banner + multiline prompt
        context = asyncio.run(multiline_prompt_async("What would you like to build?"))

    if context:
        resolved_context = resolve_initial_context_input(context, cwd=Path.cwd())
        # → @file 자동 expansion + 1MB 제한 검사
```

→ `resolve_initial_context_input` 가 `@path/to/file` 형태면 파일 내용을 읽어 inline 으로 합침 (Section 4 에서 다룬 기능). 1 MB 초과 거부.

### Sub-commands

```python
@app.command()                       # ouroboros init start
def start(...): ...

@app.command("list")                 # ouroboros init list
def list_interviews(...): ...
```

`init` Typer group 의 다른 sub-commands 는 다른 모듈에서 등록될 수도 있지만, 본 파일은 `start` 와 `list` 두 개만 노출.

## 28.4 Section 별 보강 항목

### Section 5 (Phase 2 Double Diamond) 추가 사항

- **Recovery 분기**: stagnation 감지 → `RecoveryPlanner.plan(snapshot)` → `INJECT_LATERAL_DIRECTIVE` 액션 → 같은 runtime_handle 에 새 prompt 주입. unfinished AC top 5 까지만 snapshot 에 포함.
- **Parallel fallback**: DependencyAnalyzer 실패 시 모든 AC 단일 level. 명시적 dependency 0.
- **AC outcome 5-way**: SUCCEEDED / FAILED / BLOCKED / INVALID / SATISFIED_EXTERNALLY. `--skip-completed` 가 SATISFIED_EXTERNALLY 발생.
- **Coordinator review**: level 종료 시 `coordinator.detect_file_conflicts(results)` → 충돌 시 `coordinator.run_review(...)` → LevelContext 에 review 첨부. 다음 level prompt 가 review 를 prefix 로 받음.
- **Recovery discontinuity tracking**: 같은 AC 내에서 backend session 갈리면 event 에 `recovery_discontinuity` metadata 첨부 — TUI / replay 가 turn 끊김 식별 가능.

### Section 14 (CLI surface) 추가 사항

- **`ouroboros init "<prompt>"` shorthand** = `ouroboros init start "<prompt>"`. `_DefaultStartGroup` 가 fallback resolver.
- **PM seed 자동 검출**: `~/.ouroboros/seeds/pm_seed_*.yaml` 가 있고 dev seed (`seed.json` or 다른 `*.yaml`) 가 없으면 자동 prompt. 1 개면 yes/no, 2+ 면 번호 선택. PM seed 의 YAML 본문이 dev interview 의 initial_context 로 자동 주입.
- **`ouroboros init list`** sub-command — `~/.ouroboros/data/` 에 저장된 인터뷰 세션 목록 표시 (id / status / rounds / updated_at).
- **`--orchestrator` (`-o`)**: Claude Code Max Plan (claude-agent-sdk) 사용. API key 불요.
- **`--runtime`**: workflow 실행용 backend (claude/codex/opencode/hermes). `--orchestrator` 없으면 무시.
- **`--llm-backend`**: 인터뷰/ambiguity/seed 생성용 LLM (claude_code/litellm/codex/opencode).
- **`--debug` (`-d`)**: 콘솔 verbose 로깅.
- **`--state-dir`**: 인터뷰 state 디렉토리 override (default `~/.ouroboros/data/`).
- **`--resume <id>`**: 기존 인터뷰 재개.
- **Force ambiguity bypass** (별도 플래그, 본 파일 외 정의): `FORCED_SCORE_VALUE = 0.19` → ambiguity 게이트 무조건 통과.

### Section 12 (MCP hub) 정정 — 23 도구 정확 카탈로그

`get_ouroboros_tools()` 가 반환하는 `OuroborosToolHandlers` tuple = 23 핸들러 (1차 라운드 22 → 정정). 카탈로그:

| # | 핸들러 | 기능 |
|---|---|---|
| 1 | `ExecuteSeedHandler` | seed → 동기 실행 |
| 2 | `StartExecuteSeedHandler` | seed → fire-and-forget job |
| 3 | `SessionStatusHandler` | 세션 상태 조회 |
| 4 | `JobStatusHandler` | job 상태 조회 |
| 5 | `JobWaitHandler` | job long-poll (180 sec default) |
| 6 | `JobResultHandler` | job 결과 fetch |
| 7 | `ACTreeHUDHandler` | AC 트리 시각화 (max_nodes=30) |
| 8 | `CancelJobHandler` | job 취소 |
| 9 | `QueryEventsHandler` | event_store 조회 |
| 10 | `GenerateSeedHandler` | 인터뷰 → Seed 변환 |
| 11 | `MeasureDriftHandler` | drift 측정 |
| 12 | `InterviewHandler` | dev interview 진행 |
| 13 | `EvaluateHandler` | mechanical + semantic + consensus 평가 |
| 14 | `ChecklistVerifyHandler` | acceptance criteria 검증 (`evaluate_handler` 의존) |
| 15 | `LateralThinkHandler` | 5 페르소나 lateral thinking |
| 16 | `EvolveStepHandler` | 진화 1 세대 동기 실행 |
| 17 | `StartEvolveStepHandler` | 진화 1 세대 fire-and-forget |
| 18 | `LineageStatusHandler` | 진화 lineage 상태 |
| 19 | `EvolveRewindHandler` | 진화 rewind to 세대 N |
| 20 | `CancelExecutionHandler` | 실행 취소 (in-flight) |
| 21 | `BrownfieldHandler` | brownfield (기존 코드베이스) 분석 |
| 22 | `PMInterviewHandler` | PM 인터뷰 (제품 요구사항) |
| 23 | `QAHandler` | QA judge (pass/revise/fail) |

→ Section 12 의 "21개 도구" 표 정정 필요. 별도 모듈에서 등록될 수 있는 추가 도구 (subagent envelope handler 등) 는 본 카탈로그에 포함 안 됨.

### Section 25 (magic numbers) 정정

| 상수 | Section 25 의 값 | 실제 코드 | 출처 |
|---|---|---|---|
| MAX_DEPTH (recursion) | 5 | **2** | `parallel_executor.py:DEFAULT_MAX_DECOMPOSITION_DEPTH` |
| MAX_DECOMPOSITION_DEPTH | 5 | **2** | 같음 (alias) |
| Stall timeout | 미공개 | **300 s** | `STALL_TIMEOUT_SECONDS` |
| Heartbeat | 미공개 | **30 s** | `HEARTBEAT_INTERVAL_SECONDS` |
| Max stall retries | 미공개 | **2** | `MAX_STALL_RETRIES` |
| Memory gate (min free GB) | 미공개 | **2.0 GB** | `_MIN_FREE_MEMORY_GB` |
| Memory check interval | 미공개 | **5 s** | `_MEMORY_CHECK_INTERVAL_SECONDS` |
| Memory wait max | 미공개 | **120 s** | `_MEMORY_WAIT_MAX_SECONDS` |
| Leaf result chars cap | 미공개 | **1200** | `_MAX_LEAF_RESULT_CHARS` |
| Decomposition timeout | 미공개 | **60 s** | `DECOMPOSITION_TIMEOUT_SECONDS` |
| Recovery snapshot unfinished cap | 미공개 | **5 ACs** | `runner.py:_build_recovery_snapshot` `unfinished[:5]` |
| Forced ambiguity bypass score | 미공개 | **0.19** | `cli/commands/init.py:FORCED_SCORE_VALUE` |
| Progress emit interval | 10 | 10 (확정) | `runner.py:PROGRESS_EMIT_INTERVAL` |
| Cancellation check interval | 5 | 5 (확정) | `runner.py:CANCELLATION_CHECK_INTERVAL` |

→ 1차 라운드의 Section 25 가 `llms-full.txt` 와 `architecture.md` 만 신뢰한 결과. 코드 직접 확인 후 다수 차이 발견. **이 정정 사항을 Section 25 본문에도 반영해야**.

## 28.5 미발견 영역

`runner.py` + `parallel_executor.py` + `init.py` 본체 read 후에도 정독 못 한 부분:

- `parallel_executor.py` line 2200–3479 (1279 LOC): leaf evidence 추출 detail (`_extract_leaf_evidence_lines`), composite render (`render_parallel_completion_message`, `render_parallel_verification_report`), AC scope 검증 helper 다수, decomposition LLM 프롬프트 본체 (`_try_decompose_ac` 의 system prompt 형식)
- `runner.py` line 0–300 (이미 1차 라운드에서 정독 — class init, dependencies, 도우미 함수)
- `init.py` 의 `_run_interview_loop`, `_generate_seed_from_interview` 본체 (line 100–300 부근)

이들은 US-002 / 추가 라운드에서 cover.

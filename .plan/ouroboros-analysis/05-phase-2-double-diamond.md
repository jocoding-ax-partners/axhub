# 05. Phase 2 — Double Diamond (Execution)

## 책임

Seed 의 AC 트리를 재귀 분해 + 병렬 실행 + 의존성 인지 스케줄링.

## 4 Phase 사이클

```
        * Wonder           * Design
       /  (diverge)       /  (diverge)
      /    explore       /    create
     /                  /
    * ------------ * ------------ *
     \                  \
      \    define        \    deliver
       \  (converge)      \  (converge)
        * Ontology         * Evaluation
```

| 단계 | 모드 | 역할 |
|---|---|---|
| Discover | divergent | 문제 공간 탐색 |
| Define | convergent | 핵심 문제 수렴 |
| Design | divergent | 해결책 옵션 탐색 |
| Deliver | convergent | 구현 수렴 |

## 핵심 모듈 (`src/ouroboros/execution/`)

| 파일 | 역할 |
|---|---|
| `double_diamond.py` | 4 phase 사이클 컨트롤러 |
| `decomposition.py` | 계층적 task 분해 |
| `atomicity.py` | 원자성 (1–2 파일 단일 초점) 판정 |
| `subagent.py` | 격리 서브에이전트 실행 |

## Orchestration 레이어 (`src/ouroboros/orchestrator/`, 28 파일)

가장 큰 서브패키지. 핵심:

### `runner.py` (109 KB)

`OrchestratorRunner`, `OrchestratorResult` (frozen dataclass).

#### 데이터 모델

```python
@dataclass(frozen=True, slots=True)
class OrchestratorResult:
    success: bool
    session_id: str
    execution_id: str
    summary: dict[str, Any]
    messages_processed: int
    final_message: str
    duration_seconds: float
```

#### Cancellation Registry

모듈 레벨 set + asyncio.Lock:

```python
_cancellation_registry: set[str] = set()
_cancellation_lock: asyncio.Lock = asyncio.Lock()

async def request_cancellation(session_id: str) -> None: ...
async def is_cancellation_requested(session_id: str) -> bool: ...
async def clear_cancellation(session_id: str) -> None: ...
async def get_pending_cancellations() -> frozenset[str]: ...
```

MCP cancel 도구 가 set 에 ID 추가 → runner 의 메시지 루프 가 다음 체크포인트에서 검사 → race-free.

#### System Prompt 빌드

`build_system_prompt(seed, strategy)`:
- task_type 별 strategy fragment
- goal/constraints/principles 텍스트화
- brownfield 컨텍스트 별도 섹션
- AC tracking prompt 자동 삽입
- recovery protocol prompt 추가

#### `OrchestratorError`, `ExecutionCancelledError` 예외

### `parallel_executor.py` (144 KB!) — Section 28 deep-dive

`ParallelACExecutor`. **`DEFAULT_MAX_DECOMPOSITION_DEPTH = 2`** (1차 라운드 추정 5 정정 — Section 28 참조).

상수 (Section 25 정정 후 정확):
- `MIN_SUB_ACS = 2`, `MAX_SUB_ACS = 5`
- `STALL_TIMEOUT_SECONDS = 300` (5분 무활동 → 포기)
- `HEARTBEAT_INTERVAL_SECONDS = 30`
- `MAX_STALL_RETRIES = 2`
- `_MIN_FREE_MEMORY_GB = 2.0` (메모리 게이트)
- `_MEMORY_CHECK_INTERVAL_SECONDS = 5`, `_MEMORY_WAIT_MAX_SECONDS = 120`
- `_MAX_LEAF_RESULT_CHARS = 1200` (leaf evidence 절단)
- `DECOMPOSITION_TIMEOUT_SECONDS = 60`

Leaf evidence 추출: `_extract_leaf_evidence_lines()`, `_truncate_text()` (`_MAX_LEAF_RESULT_CHARS`).

Decomposition depth 경고: `_collect_decomposition_depth_warning_paths()`.

Render 함수: `render_parallel_verification_report()`, `render_parallel_completion_message()`, `_render_ac_section()`.

자원 검사: `_get_available_memory_gb()`.

#### AC outcome 5-way 분류

```python
class ACExecutionOutcome(StrEnum):
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    BLOCKED = "blocked"               # 의존성 실패로 skip
    INVALID = "invalid"               # dependency graph 누락
    SATISFIED_EXTERNALLY = "satisfied_externally"  # --skip-completed
```

#### Stage 실행 흐름

`execute_parallel()`:
1. RC3 checkpoint 복구 시도 (`_checkpoint_store.load(seed_id)`)
2. `expected_indices = set(range(total_acs))` — dependency graph 누락/초과 검증
3. `outer_tg.start_soon(_resilient_progress_emitter)` — 백그라운드 progress emitter
4. `for stage in execution_plan.stages`:
   - `executable / blocked / externally_satisfied` 3-way 분류 (의존성 검증)
   - `for batch in stage_batches`: `_execute_ac_batch()` 병렬 실행
   - 결과 분류 (BaseException / `_STALL_SENTINEL` / 정상)
   - **Coordinator review**: `_coordinator.detect_file_conflicts()` → 충돌 시 `run_review()` → `LevelContext` 에 review 첨부 → 다음 level prompt prefix
   - 매 level 끝 RC3 checkpoint 저장
5. `outer_tg.cancel_scope.cancel()` — 백그라운드 emitter 종료

#### Recursive AC execution (`_execute_single_ac`)

```python
if self._enable_decomposition and depth < self._max_decomposition_depth:
    sub_acs = await self._try_decompose_ac(...)
    if sub_acs and len(sub_acs) >= MIN_SUB_ACS:
        # 재귀: depth+1 로 sub_acs 다시 _execute_single_ac
        for idx, sub_ac in enumerate(sub_acs):
            sub_result = await self._execute_single_ac(
                ac_index=ac_index,         # 부모 ac_index 유지
                ac_content=sub_ac,
                depth=depth + 1,
                is_sub_ac=True,
                parent_ac_index=ac_index,
                sub_ac_index=idx,
                ...
            )
        # leaf result 결합 → composite ACExecutionResult
```

depth 0 (top) → depth 1 (sub) → depth 2 강제 atomic. 더 깊이 원하면 `max_decomposition_depth` 파라미터 override.

#### Runtime handle scope normalization

`_normalize_ac_runtime_handle()` — 각 AC 가 자기 scope 의 RuntimeHandle 만 reuse 보장. 다른 scope 의 native_session_id 가 흘러 들어오면 `scrub_resume_state=True` 로 metadata 만 남기고 resume state 폐기. AC 격리.

#### Recovery discontinuity tracking

같은 AC 내에서 backend session 이 새로 시작되면 (예: Claude 401 후 재인증) event 에 `recovery_discontinuity` 메타 첨부 — TUI/replay 가 turn 끊김 식별.

### `coordinator.py`

레벨 코디네이터 — 같은 레벨의 형제 AC 들이 동일 파일 충돌 시 review prompt 발동.

```python
def derive_coordinator_tools(runtime_backend: str | None) -> list[str]: ...

class FileConflict: ...
class CoordinatorReview: ...
class LevelCoordinator:
    def _collect_file_modifications(...): ...
    def _build_review_prompt(...): ...
    def _parse_review_response(...): ...
```

### `dependency_analyzer.py`

`DependencyAnalyzer` — AC 간 의존 그래프 추론. LLM-assisted (선택), 구조 기반 (필수).

### `runtime_factory.py`

`create_agent_runtime(backend, ...)`. Alias 정규화:
- `claude` / `claude_code` → `claude`
- `codex` / `codex_cli` → `codex`
- `opencode` / `opencode_cli` → `opencode`
- `hermes` / `hermes_cli` → `hermes`

Resolution 순서:
1. `OUROBOROS_AGENT_RUNTIME` env var
2. `orchestrator.runtime_backend` in `~/.ouroboros/config.yaml`
3. 명시적 `backend=` 파라미터

미지원 alias → `ValueError`.

### `adapter.py` — `ClaudeAgentAdapter` (1595 LOC)

가장 정교한 어댑터. 11번 섹션 별도.

### `command_dispatcher.py`

`create_codex_command_dispatcher(cwd, runtime_backend, llm_backend)` — Codex 가 슬래시 명령 받으면 deterministic MCP dispatch 변환.

### `policy.py`

`PolicyContext`, `PolicyDecision`, `PolicyExecutionPhase`, `PolicySessionRole`, `evaluate_capability_policy()`.

도구 사용 정책: phase 별 / role 별 어떤 capability 허용할지 결정.

### `mcp_tools.py`

`MCPToolProvider`, `SessionToolCatalog`, `assemble_session_tool_catalog()`, `enumerate_runtime_builtin_tool_definitions()`, `serialize_tool_catalog()`.

### `rate_limit.py`

`SharedRateLimitBucket` — RPM/TPM 공유 버킷.

상수:
- `DEFAULT_ANTHROPIC_RPM_CEILING`, `DEFAULT_ANTHROPIC_TPM_CEILING`
- `RATE_LIMIT_HEARTBEAT_SECONDS = 5`
- `RATE_LIMIT_MAX_WAIT_SECONDS`

Env override: `OUROBOROS_ANTHROPIC_RPM_CEILING`, `OUROBOROS_ANTHROPIC_TPM_CEILING` (0 = 무제한).

핵심 트릭 — `force_reserve` fallback:

```
타임아웃 초과 후 그냥 capacity 예약 — 안 그러면 동시 timeout-fallback 들이 모두 bucket bypass → N× RPM burst → upstream 폭발 (starvation 보다 더 나쁨)
```

`SharedRateLimitBucket.acquire(estimated_tokens)` — async, 대기 시 heartbeat AgentMessage emit.

`estimate_runtime_request_tokens(prompt, system_prompt)` — 토큰 추정.

### `capabilities.py`

`CapabilityGraph`, `build_capability_graph()`, `serialize_capability_graph()`. 도구 의존성 그래프.

### `control_plane.py`

`build_control_plane_state()`, `serialize_control_plane_state()`. 멀티 RuntimeHandle 의 control 채널 추상화.

### `execution_strategy.py`

`ExecutionStrategy`, `get_strategy(task_type)`, prompt fragment 차별화.

### `level_context.py`

`LevelContext` — depth 3+ 에서 context 500 chars 절단 (`COMPRESSION_DEPTH = 3`).

### `session.py`

`SessionRepository`, `SessionStatus`, `SessionTracker`.

### `runtime_message_projection.py`

backend-neutral message projection: `message_tool_input()`, `message_tool_name()`, `normalized_message_type()`, `project_runtime_message()`.

### `workflow_state.py`

`coerce_ac_marker_update()`, `get_ac_tracking_prompt()` — AC 트래킹 system prompt.

### `heartbeat.py`

RuntimeHandle heartbeat — 장기 실행 시 alive 신호.

## 재귀 분해

각 AC → Discover + Define → atomicity 검사:
- **Atomic** (single-focused, 1–2 파일) → Design + Deliver 진행
- **Non-atomic** → 2-5 child AC 로 분해 → 재귀

제약:
- **`DEFAULT_MAX_DECOMPOSITION_DEPTH = 2`** ⚠ (Section 28 정정. 1차 라운드 5 는 docs/추정값. 실제 코드는 2)
- `COMPRESSION_DEPTH = 3` (depth 3+ 에서 context 500 chars)
- 자식들 의존성 정렬 후 레벨별 병렬 실행
- `STALL_TIMEOUT_SECONDS = 300` 무활동 시 AC 포기

## Recovery Protocol

`recovery.py`:

```python
class RecoveryActionKind(StrEnum):
    INJECT_LATERAL_DIRECTIVE = ...     # 5 페르소나 중 1 의 prompt 주입
    # 그 외 NOOP, ABANDON 등

class RecoveryPlanner:
    def plan(self, snapshot: RecoverySnapshot) -> RecoveryAction: ...

class RecoverySnapshot:
    problem_context: str               # Goal + unfinished AC top 5 + previous final[:1000]
    current_approach: str
    messages_processed: int
    completed_count: int
    total_count: int
    final_error: str
    used_personas: tuple[ThinkingPersona, ...]   # 이미 시도한 페르소나
    interventions_used: int

def create_recovery_applied_event(...): ...
def get_run_recovery_protocol_prompt() -> str: ...
```

System prompt 에 자동 삽입 + `runner.py` 메시지 루프 끝에서 실패 시 호출.

흐름 (Section 28 정독 결과):
1. 메시지 루프 종료 후 `success=False` 면 `RecoveryPlanner.plan(snapshot)` 호출
2. `recovery_action.kind == INJECT_LATERAL_DIRECTIVE` 이고 `directive` + `persona` 있으면:
   - `recovery_interventions_used += 1`, `recovery_personas.append(action.persona.value)`
   - `create_recovery_applied_event(...)` 발행
   - **같은 runtime_handle 에 새 prompt (lateral directive) 주입** → `_consume_task_stream(prompt=action.directive, resume_handle=runtime_handle)` 다시 실행
3. unfinished AC 의 첫 5 개만 snapshot.problem_context 에 포함 (`runner.py:_build_recovery_snapshot()` 의 `unfinished[:5]`)
4. 5 페르소나 한 번씩 (`max_lateral_attempts = 5`) → 모두 소진 시 ABANDON

## Resume 3-way 분기 (Section 28)

`runner.py:resume_session()` 종료 시:

| success | recoverable_resume_failure | session 상태 |
|---|---|---|
| True | (n/a) | COMPLETED — `mark_completed(...)` |
| False | True | PAUSED — `mark_paused(reason, resume_hint="Retry the same --resume session after fixing the runtime/tooling issue.")` |
| False | False | FAILED — `mark_failed(...)` |

`recoverable_resume_failure` = `_is_recoverable_resume_failure(message)` — 일시적 backend/네트워크 오류만 PAUSED 로 mark, 실 fix 후 retry 가능.

## Parallel fallback

`_execute_parallel()` 의 DependencyAnalyzer 가 실패하면:

```python
all_indices = tuple(range(len(seed.acceptance_criteria)))
dependency_graph = DependencyGraph(
    nodes=tuple(ACNode(index=i, content=ac, depends_on=()) for i, ac in enumerate(seed.acceptance_criteria)),
    execution_levels=(all_indices,) if all_indices else (),
)
```

→ 모든 AC 가 단일 level 에서 병렬. 명시적 의존성 0. 이 fallback 은 `architecture.md` 미공개.

## Subagent 격리 (`execution/subagent.py`)

별도 LLM 컨텍스트 — 부모 컨텍스트 오염 방지. `tests/unit/execution/test_subagent_isolation.py` 가 검증.

## 의존 컴포넌트

- ProviderFactory — LLM 호출
- EventStore — `execution.ac.*` 이벤트
- AC Tree — 계층 구조
- DependencyAnalyzer — 그래프
- LevelCoordinator — 충돌 review

## Phase 2 ↔ 다른 Phase 연결

- In ← Phase 0 (Seed.acceptance_criteria → AC tree)
- In ← Phase 1 (선택된 tier 로 LLM 호출)
- Out → Phase 4 (실행 산출물 → Stage 1 mechanical)
- Re-trigger ← Phase 3 (stagnation → lateral persona prompt)

# 35. Residual Modules — Round 3 gap fill

> Round 3 review pass — Round 2 의 gap-finder 가 발견한 critical gap 중 Section 34 가 채우지 못한 잔여 영역. orchestrator 잔여 + MCPClientManager + verification/extractor + execution_runtime_scope.

## 35.1 `orchestrator/parallel_executor_models.py` (270 LOC) — AC outcome enum 의 home

> Section 28 / 05 에서 reference 만 했던 enum 본체. 단일 source of truth.

### `ACExecutionOutcome` enum (line 24-31)

```python
class ACExecutionOutcome(str, Enum):
    """Normalized outcome for a single AC execution."""

    SUCCEEDED = "succeeded"
    SATISFIED_EXTERNALLY = "satisfied_externally"
    FAILED = "failed"
    BLOCKED = "blocked"
    INVALID = "invalid"
```

→ **5-way 정확 확인** (Section 28 의 주장 일치). string-enum (`str, Enum`) — JSON 직렬화 호환.

### `ACExecutionResult` dataclass (line 34-112)

```python
@dataclass(frozen=True, slots=True)
class ACExecutionResult:
    ac_index: int
    ac_content: str
    success: bool
    messages: tuple[AgentMessage, ...] = field(default_factory=tuple)
    final_message: str = ""
    error: str | None = None
    duration_seconds: float = 0.0
    session_id: str | None = None
    retry_attempt: int = 0
    is_decomposed: bool = False
    sub_results: tuple[ACExecutionResult, ...] = field(default_factory=tuple)
    depth: int = 0
    decomposition_depth_warning: bool = False
    outcome: ACExecutionOutcome | None = None
    runtime_handle: RuntimeHandle | None = None
```

**핵심 invariant** (`__post_init__` line 73-76):
```python
def __post_init__(self) -> None:
    """Normalize outcome so callers do not infer from error strings."""
    if self.outcome is None:
        object.__setattr__(self, "outcome", self._infer_outcome())
```
→ frozen dataclass 라 `object.__setattr__` 으로 우회 (Python 의 frozen 우회 표준 idiom).

### `_infer_outcome()` 알고리즘 (line 78-87)

```python
def _infer_outcome(self) -> ACExecutionOutcome:
    if self.success:
        return ACExecutionOutcome.SUCCEEDED

    error_text = (self.error or "").lower()
    if "not included in dependency graph" in error_text:
        return ACExecutionOutcome.INVALID
    if "skipped: dependency failed" in error_text or "blocked: dependency" in error_text:
        return ACExecutionOutcome.BLOCKED
    return ACExecutionOutcome.FAILED
```

→ **error string-based inference**. 위험 — error message wording 변경 시 outcome classification 실패. 명시적 outcome enum 으로 호출자가 setting 권장 (`outcome: ACExecutionOutcome | None = None`).

→ `SATISFIED_EXTERNALLY` 는 inference 에서 안 나옴 — 호출자가 명시적으로 설정 필요.

### convenience properties (line 89-112)

| property | 의미 |
|---|---|
| `is_blocked` | outcome == BLOCKED |
| `is_satisfied_externally` | outcome == SATISFIED_EXTERNALLY |
| `is_failure` | outcome == FAILED (NOT BLOCKED/INVALID/SATISFIED) |
| `is_invalid` | outcome == INVALID |
| `attempt_number` | retry_attempt + 1 (1-based human readable) |

### `StageExecutionOutcome` enum (line 115-121)

```python
class StageExecutionOutcome(str, Enum):
    """Aggregate outcome for a serial execution stage."""

    SUCCEEDED = "succeeded"
    FAILED = "failed"
    BLOCKED = "blocked"
    PARTIAL = "partial"
```

→ **4-way** (NOT 5). Stage = level (parallel batch) 의 집계 결과. `PARTIAL` 추가됨 — 일부 AC 성공 + 일부 실패 시.

### `ParallelExecutionStageResult` aggregator (line 124-160)

```python
@dataclass(frozen=True, slots=True)
class ParallelExecutionStageResult:
    stage_index: int
    ac_indices: tuple[int, ...]
    results: tuple[ACExecutionResult, ...] = field(default_factory=tuple)
    started: bool = True
    coordinator_review: CoordinatorReview | None = None
```

`success_count` property:
```python
return sum(
    1
    for result in self.results
    if result.outcome in {
        ACExecutionOutcome.SUCCEEDED,
        ACExecutionOutcome.SATISFIED_EXTERNALLY,
    }
)
```
→ SATISFIED_EXTERNALLY 도 success 로 카운트. Section 5 의 AC outcome 처리 정확성.

`externally_satisfied_count` — 별도 metric (성공 중 외부 satisfaction 비율).

## 35.2 `orchestrator/execution_runtime_scope.py` (214 LOC) — AC 격리 메커니즘

> Section 28 의 "Runtime handle scope normalization" 의 코드 본체.

### `ExecutionRuntimeScope` (line 13-31)

```python
@dataclass(frozen=True, slots=True)
class ExecutionRuntimeScope:
    """A stable identity/path pair for persisted execution runtime state."""

    aggregate_type: str
    aggregate_id: str
    state_path: str
    retry_attempt: int = 0
```

**Invariant** (post_init): `retry_attempt < 0` raise `ValueError`.

### `ACRuntimeIdentity` (line 34-98)

```python
@dataclass(frozen=True, slots=True)
class ACRuntimeIdentity:
    runtime_scope: ExecutionRuntimeScope
    ac_index: int | None = None
    parent_ac_index: int | None = None
    sub_ac_index: int | None = None
    scope: str = "ac"
    session_role: str = "implementation"
```

**핵심 properties**:
- `ac_id` = runtime_scope.aggregate_id (retry 간 stable)
- `session_scope_id` = same as ac_id (같은 AC 의 retry 간 session 재사용)
- `session_attempt_id` = `f"{session_scope_id}_attempt_{attempt_number}"` (retry 마다 unique)
- `cache_key` = session_attempt_id (same-attempt resume 용)

→ **격리 hierarchy**: ac_id (AC 식별) > session_scope_id (AC 의 모든 retry 공유) > session_attempt_id (한 retry 의 unique ID)

### `to_metadata()` 직렬화 (line 80-98)

```python
metadata: dict[str, object] = {
    "ac_id": self.ac_id,
    "scope": self.scope,
    "session_role": self.session_role,
    "retry_attempt": self.retry_attempt,
    "attempt_number": self.attempt_number,
    "session_scope_id": self.session_scope_id,
    "session_attempt_id": self.session_attempt_id,
    "session_state_path": self.session_state_path,
}
if self.parent_ac_index is not None:
    metadata["parent_ac_index"] = self.parent_ac_index
if self.sub_ac_index is not None:
    metadata["sub_ac_index"] = self.sub_ac_index
if self.ac_index is not None and self.parent_ac_index is None:
    metadata["ac_index"] = self.ac_index
```

→ runtime handle 의 metadata 에 모든 식별자 직렬화. 부분 None 은 omit (sub-AC 일 때 ac_index 없음, top-level AC 일 때 parent/sub 없음).

### `_normalize_scope_segment()` (line 101-104)

```python
def _normalize_scope_segment(value: str, *, fallback: str) -> str:
    """Normalize dynamic identifiers for safe inclusion in scope metadata."""
    normalized = re.sub(r"[^a-zA-Z0-9_-]+", "_", value).strip("_")
    return normalized or fallback
```

→ 모든 non-alphanumeric/underscore/hyphen → `_` 변환. 빈 결과는 fallback. 경로 injection 방지.

### `build_ac_runtime_scope()` (line 107-159)

3-way branch:
1. **Sub-AC** (is_sub_ac=True):
   - `aggregate_id = f"sub_ac_{parent_ac_index}_{sub_ac_index}"`
   - `state_path = "execution.acceptance_criteria.ac_{parent}.sub_acs.sub_ac_{sub}.implementation_session"`
2. **Top-level AC**:
   - `aggregate_id = f"ac_{ac_index}"`
   - `state_path = f"execution.acceptance_criteria.ac_{ac_index}.implementation_session"`
3. **Workflow scope optional**:
   - `execution_context_id` 가 있으면 `aggregate_id` 와 `state_path` 가 `workflows.{scope}.` prefix 추가

→ multi-workflow / sub-AC / retry 모두 같은 AC 라도 분리된 state path 가짐.

### `build_level_coordinator_runtime_scope()` (line 188-205)

```python
return ExecutionRuntimeScope(
    aggregate_type="execution",
    aggregate_id=f"{execution_scope}_level_{level_number}_coordinator_reconciliation",
    state_path=(
        "execution.workflows."
        f"{execution_scope}.levels.level_{level_number}."
        "coordinator_reconciliation_session"
    ),
)
```

→ Coordinator 의 reconciliation work 도 level 별로 격리.

## 35.3 `verification/extractor.py` (167 LOC) — `AssertionExtractor`

> Section 7 (Evaluation) + Section 29 의 `SpecVerifier` 의 동반. AC → SpecAssertion 변환 LLM caller.

### 4-tier classification (`_SYSTEM_PROMPT` line 29-59)

| Tier | 의미 | 예시 |
|---|---|---|
| `t1_constant` | regex 로 source 에서 찾을 수 있는 specific value | "WARMUP_FRAMES=10", "30 second timeout", "5 retries" |
| `t2_structural` | 특정 file/class/interface/function 존재 | "CameraProvider interface", "tests directory", "--verbose flag" |
| `t3_behavioral` | 코드/테스트 실행 필요 | "3 calls return median score", "handles errors gracefully" |
| `t4_unverifiable` | 주관적 / 인간 판단 | "UX feels natural", "code is clean" |

→ **conservative 원칙**: 불확실하면 t3_behavioral (NOT t1/t2). Section 29 의 SpecVerifier 가 t1/t2 만 자동화 — t3/t4 는 manual / LLM evaluation.

### LLM call structure (line 102-117)

```python
config = CompletionConfig(
    model=self.model,
    temperature=0.0,         # deterministic
    max_tokens=4096,
)

result = await self.llm_adapter.complete(messages, config)
```

→ temperature=0.0 — reproducibility. 4096 tokens 이면 ~30 AC 정도 capacity.

### LRU cache (line 70-75 + 119-122)

```python
max_cache_size: int = 64
_cache: OrderedDict[str, tuple[SpecAssertion, ...]] = field(...)

# Cache eviction
while len(self._cache) > self.max_cache_size:
    self._cache.popitem(last=False)
```

→ seed_id 기반 caching. 같은 seed 가 multiple generation 에 걸쳐 재사용되면 LLM call 1 번만. cache 가 64 seed 까지 — 그 이상은 oldest 제거.

### Response parsing (line 125-167)

```python
cleaned = content.strip()
if cleaned.startswith("```"):
    lines = cleaned.split("\n")
    cleaned = "\n".join(lines[1:-1])

data = json.loads(cleaned)
if not isinstance(data, list):
    return ()  # graceful empty
```

→ markdown fence 제거 (LLM 이 가끔 ```json ... ``` 로 감쌈).
→ JSON parse 실패 / non-list → empty tuple (graceful degradation).
→ tier value 가 invalid 하면 T4_UNVERIFIABLE 로 default.

### 핵심 design 원칙

1. **Cost optimization**: LRU cache 로 redundant LLM call 방지
2. **Graceful degradation**: parse 실패 → empty (assertion 없음, verifier 가 자동 skip)
3. **Reproducibility**: temperature=0.0 + seed_id cache → same input → same output
4. **Conservative classification**: 불확실하면 t3 (manual) — false-positive automation 회피

## 35.4 `mcp/client/manager.py` (684 LOC) — `MCPClientManager`

> Section 12 (MCP hub) 의 client side. Section 34 의 MCPBridge 가 wrap.

### `ConnectionState` enum (line 35-41)

```python
class ConnectionState(StrEnum):
    DISCONNECTED = "disconnected"
    CONNECTING = "connecting"
    CONNECTED = "connected"
    UNHEALTHY = "unhealthy"
    ERROR = "error"
```

→ **5-state**. UNHEALTHY 는 health check 실패 (recoverable), ERROR 는 fatal.

### `ServerConnection` dataclass (line 45-63)

```python
@dataclass(frozen=True, slots=True)
class ServerConnection:
    config: MCPServerConfig
    adapter: MCPClientAdapter
    state: ConnectionState = ConnectionState.DISCONNECTED
    last_error: str | None = None
    tools: tuple[MCPToolDefinition, ...] = field(default_factory=tuple)
    resources: tuple[MCPResourceDefinition, ...] = field(default_factory=tuple)
```

→ frozen — state 변경 시 새 instance 생성 (immutable update).

### `MCPClientManager.__init__` (line 97-116)

```python
def __init__(
    self,
    *,
    max_retries: int = 3,
    health_check_interval: float = 60.0,
    default_timeout: float = 30.0,
) -> None:
    self._max_retries = max_retries
    self._health_check_interval = health_check_interval
    self._default_timeout = default_timeout
    self._connections: dict[str, ServerConnection] = {}
    self._health_check_task: asyncio.Task[None] | None = None
    self._lock = asyncio.Lock()
```

**기본 설정**:
- `max_retries = 3` — 연결 시도 횟수 (재시도 외)
- `health_check_interval = 60.0s` — 1분마다 health check
- `default_timeout = 30.0s` — 개별 op timeout

→ Section 34 의 `MCPBridge.__post_init__` 가 이 값들을 config 에서 override.

### Features (docstring line 67-94)

> "Connection pooling: Reuses connections to servers"
> "Health checks: Periodic checks for connection health"
> "Per-request timeouts: Individual timeout per operation"
> "Tool aggregation: Access all tools across servers"
> "Auto-reconnection: Attempts to reconnect on failure"

### Background task — health check

`_health_check_task: asyncio.Task[None] | None` — manager 시작 시 background task 가 health_check_interval 마다:
1. 모든 connection ping
2. 실패한 것 UNHEALTHY 로 전환
3. recovery 시도 (max_retries)
4. 영구 실패 시 ERROR

### Lock pattern

`asyncio.Lock` — connection state 변경 시 race condition 방지. add_server / connect / disconnect / call_tool 모두 lock 안에서.

## 35.5 Cross-module 발견

### Section 28 + 35.1 cross-validation

Section 28 의 AC outcome 5-way 주장 = parallel_executor_models.py:24-31 직접 확인. **정확**.

### Section 7 + 35.3 cross-validation

Section 7 의 4-tier verification (T1/T2/T3/T4) = extractor.py 의 SYSTEM_PROMPT 직접 확인. **정확**.

### Section 12 + 34 + 35.4 chain

```
MCPBridge (34)
  └── MCPClientManager (35.4)
       └── ServerConnection
            └── MCPClientAdapter (mcp/client/adapter.py — 미독)
```

→ 3-layer. Section 12 가 top, 34 가 middle, 35.4 가 bottom. adapter.py 본체는 다음 round.

### Frozen dataclass 패턴 (반복)

이번 round 의 모든 dataclass:
- `ACExecutionResult` — frozen, slots
- `ACExecutionOutcome` — string-enum
- `ParallelExecutionStageResult` — frozen, slots
- `ExecutionRuntimeScope` — frozen, slots
- `ACRuntimeIdentity` — frozen, slots
- `ServerConnection` — frozen, slots

→ Ouroboros 의 일관 immutability 원칙. 변경 = 새 instance 생성.

### LRU cache 패턴 (반복)

- `agents/loader.py` — `@functools.lru_cache(maxsize=64)` (Section 13)
- `evaluation/extractor.py` — `OrderedDict` + `popitem(last=False)` (이번 35.3, maxsize=64)
- `opencode/plugin/ouroboros-bridge.ts` — `dupe()` 의 LRU eviction (Section 16)

→ 다른 storage 형식이지만 같은 LRU 의도. Cache size 64 가 standard.

## 35.6 미발견 영역 (Round 3 이후 잔여)

### Production code
- `mcp/client/adapter.py` (MCPClientAdapter 본체 — 35.4 의 한 layer 아래)
- `mcp/server/adapter.py` (FastMCP 어댑터)
- `command_dispatcher.py` 동적 라우팅
- 21 잔여 orchestrator file 중 여전히 미독:
  - `orchestrator/heartbeat.py` (이번엔 시그니처만 봄)
  - `orchestrator/coordinator.py` (CoordinatorReview 의 home — Section 5 가 reference)
  - `orchestrator/dependency_analyzer.py`
  - `orchestrator/level_context.py` (parallel_executor_models 가 import)
  - `orchestrator/policy.py` (SandboxClass 의 home)
  - 등...
- `router/dispatch.py` 본체 (이번엔 시그니처만)
- `router/command_parser.py`

### TUI
- `dashboard_v3.py` 32.6 KB 본문 (Section 34 가 metadata 만)
- 8 widget body
- 7 screen body
- Rust views 의 helper body

### 테스트
- 270+ 테스트 case 함수 본체
- Bun bridge.test.ts
- Rust cargo test

## 35.7 Round 3 deliverable

- ✅ Section 35 — 4 critical residual file 본체 분석:
  - `parallel_executor_models.py` (AC outcome enum home + 5-way 직접 확인)
  - `execution_runtime_scope.py` (AC 격리 메커니즘 본체)
  - `verification/extractor.py` (4-tier classification + LRU cache)
  - `mcp/client/manager.py` (5-state ConnectionState + health check + lock pattern)
- ✅ Section 28 의 AC outcome 5-way 주장 cross-validate (직접 확인)
- ✅ Section 7 의 4-tier verification 주장 cross-validate
- ✅ Section 12 + 34 + 35.4 의 MCP layer chain 분석
- ✅ Frozen dataclass + LRU cache 패턴 반복성 발견 (Ouroboros 의 design idiom)

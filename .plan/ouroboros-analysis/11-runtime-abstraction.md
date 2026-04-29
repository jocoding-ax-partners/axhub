# 11. Runtime Abstraction Layer

## 책임

Workflow orchestration ↔ agent runtime 분리. 4 백엔드 (Claude/Codex/OpenCode/Hermes) 동등 추상화. 같은 Seed 가 다른 엔진에서 실행 가능.

## AgentRuntime Protocol

`src/ouroboros/orchestrator/adapter.py:668`:

```python
class AgentRuntime(Protocol):
    @property
    def runtime_backend(self) -> str: ...                   # "claude" | "codex_cli" | ...
    
    @property
    def llm_backend(self) -> str | None: ...                # v0.28.6+, None → fallback to runtime_backend
    
    @property
    def working_directory(self) -> str | None: ...
    
    @property
    def permission_mode(self) -> str | None: ...            # "acceptEdits" | "bypassPermissions" | "default"
    
    def execute_task(
        self,
        prompt: str,
        tools: list[str] | None = None,
        system_prompt: str | None = None,
        resume_handle: RuntimeHandle | None = None,
        resume_session_id: str | None = None,                 # deprecated, use resume_handle
    ) -> AsyncIterator[AgentMessage]: ...
    
    async def execute_task_to_result(...) -> Result[TaskResult, ProviderError]: ...
```

Protocol → 구조적 서브타이핑. 새 어댑터 추가 시 `runtime_factory.py` 한 곳만 수정.

## RuntimeHandle (frozen dataclass)

`adapter.py:373`:

```python
@dataclass(frozen=True, slots=True)
class RuntimeHandle:
    backend: str                                              # canonical
    kind: str = "agent_runtime"
    native_session_id: str | None = None                      # backend-native ID
    conversation_id: str | None = None                        # 영구 thread ID
    previous_response_id: str | None = None                   # turn-chaining API
    transcript_path: str | None = None                        # CLI runtime 경로
    cwd: str | None = None
    approval_mode: str | None = None
    updated_at: str | None = None                              # ISO timestamp
    metadata: dict[str, Any] = field(default_factory=dict)
    _observe_callback: RuntimeHandleObserver | None = None
    _terminate_callback: RuntimeHandleTerminator | None = None
```

## 계산 속성 (computed)

```python
@property
def server_session_id(self) -> str | None:
    return _optional_str(self.metadata.get("server_session_id"))

@property
def ac_id(self) -> str | None:
    return _optional_str(self.metadata.get("ac_id"))

@property
def session_scope_id(self) -> str | None: ...

@property
def session_attempt_id(self) -> str | None: ...

@property
def resume_session_id(self) -> str | None:
    """우선순위: native_session_id → server_session_id"""

@property
def control_session_id(self) -> str | None:
    """우선순위: server_session_id → native_session_id"""

@property
def runtime_event_type(self) -> str | None: ...

@property
def lifecycle_state(self) -> str:
    return _runtime_handle_lifecycle_state(self.runtime_event_type, has_session_id=...)

@property
def is_terminal(self) -> bool:
    return self.lifecycle_state in {"cancelled", "completed", "failed", "terminated"}

@property
def can_resume(self) -> bool:
    return self.resume_session_id is not None

@property
def can_observe(self) -> bool:
    return self._observe_callback or self.control_session_id or self.resume_session_id

@property
def can_terminate(self) -> bool:
    return self._terminate_callback is not None and not self.is_terminal
```

## Lifecycle State 매핑 (`_RUNTIME_LIFECYCLE_STATE_BY_EVENT_TYPE`)

```
runtime.connected     → connecting
runtime.ready         → ready
session.bound         → ready
session.created       → starting
session.ready         → ready
session.started       → running
session.resumed       → running
thread.started        → running
result.completed      → running
turn.completed        → running
run.completed         → completed
session.completed     → completed
task.completed        → completed
error                 → failed
run.failed            → failed
session.failed        → failed
task.failed           → failed
```

추가 keyword-based fallback:
- `permission`/`approval` → `awaiting_permission`
- `cancelled`/`canceled` → `cancelled`
- `terminated` → `terminated`
- `failed` → `failed`
- `completed` → `completed`
- `connected/created/bound/ready/resumed/started` → `running`

## Backend Alias 정규화

`_RUNTIME_HANDLE_BACKEND_ALIASES`:
```python
{
    "claude": "claude",
    "claude_code": "claude",
    "codex": "codex_cli",
    "codex_cli": "codex_cli",
    "opencode": "opencode",
    "opencode_cli": "opencode",
    "hermes": "hermes_cli",
    "hermes_cli": "hermes_cli",
}
```

`_normalize_runtime_handle_selector(selector, *, field_name)`:
- None → None
- 문자열 → strip + lower → alias 검색
- 매치 안 되면 ValueError

`_resolve_runtime_handle_backend(*, backend, provider=None)`:
- backend 와 provider 둘 다 정규화
- 둘 다 None → ValueError
- 둘 다 있고 다르면 → ValueError ("backend/provider conflict")
- 하나만 있거나 같으면 → 그 값

`__post_init__` 자동 호출 — 생성 시 자동 정규화.

## Persistence 분리

```python
def to_dict(self) -> dict[str, Any]:
    """Progress 영속화용 — 전체 메타데이터"""

def to_persisted_dict(self) -> dict[str, Any]:
    """Event/session 영속화용 — OpenCode 만 화이트리스트 적용"""
```

OpenCode `_OPENCODE_PERSISTED_METADATA_KEYS` 화이트리스트 (16종):
```
ac_id, ac_index, attempt_number, execution_id, level_number, parent_ac_index,
recovery_discontinuity, retry_attempt, scope, server_session_id,
session_attempt_id, session_role, session_scope_id, session_state_path,
capability_graph, control_plane, sub_ac_index, tool_catalog,
turn_id, turn_number
```

→ stored event minimal + resume-safe.

`to_session_state_dict()` = `to_persisted_dict()` (OpenCode 동일).

## Bind Controls

```python
def bind_controls(
    self,
    *,
    observe_callback: RuntimeHandleObserver | None = None,
    terminate_callback: RuntimeHandleTerminator | None = None,
) -> RuntimeHandle:
    """Live observe/terminate 콜백 바인딩 — 영속 데이터엔 영향 없음 (replace 사용)"""
```

`bind_controls` 는 frozen dataclass 의 `replace` 로 새 인스턴스 반환.

## Live Operation

```python
async def observe(self) -> dict[str, Any]:
    if self._observe_callback:
        return await self._observe_callback(self)
    return self.snapshot()

async def terminate(self) -> bool:
    if not self.can_terminate or not self._terminate_callback:
        return False
    return await self._terminate_callback(self)
```

## AgentMessage (frozen dataclass)

```python
@dataclass(frozen=True, slots=True)
class AgentMessage:
    type: str                                                  # "assistant"|"user"|"tool"|"result"|"system"
    content: str
    tool_name: str | None = None
    data: dict[str, Any] = field(default_factory=dict)
    resume_handle: RuntimeHandle | None = None
    
    @property
    def is_final(self) -> bool:
        return self.type == "result"
    
    @property
    def is_error(self) -> bool:
        return self.data.get("subtype") == "error"
```

## TaskResult (frozen)

```python
@dataclass(frozen=True, slots=True)
class TaskResult:
    success: bool
    final_message: str
    messages: tuple[AgentMessage, ...]
    session_id: str | None = None
    resume_handle: RuntimeHandle | None = None
```

## 4 어댑터 비교

| 어댑터 | 모듈 | LOC | 기반 | 특징 |
|---|---|---|---|---|
| `ClaudeAgentAdapter` | `orchestrator/adapter.py` | 1595 | claude-agent-sdk + Claude Code CLI | 가장 정교, MCP delegation hook, shared rate limit, 3 retry exponential, MAX_RETRIES=3, RETRY_WAIT_INITIAL=1.0, RETRY_WAIT_MAX=10.0 |
| `CodexCliRuntime` | `orchestrator/codex_cli_runtime.py` | mid | OpenAI Codex CLI subprocess | NDJSON parser, skill-command interception (`command_dispatcher.py`), recursion guard (`codex/cli_policy.py`) |
| `OpenCodeRuntime` | `orchestrator/opencode_runtime.py` | mid | OpenCode CLI | subprocess 모드 하드코딩 (plugin 모드 = MCP 서버 컨텍스트 전용), `opencode_event_normalizer.py` 로 이벤트 정규화 |
| `HermesCliRuntime` | `orchestrator/hermes_runtime.py` | mid | Hermes CLI | 가장 신규 |

## ClaudeAgentAdapter 핵심 디테일

### Initialization

```python
def __init__(
    self,
    api_key: str | None = None,                    # ANTHROPIC_API_KEY 또는 Claude Code CLI 인증
    permission_mode: str = "acceptEdits",
    model: str | None = None,                       # claude-sonnet-4-6 등
    cwd: str | Path | None = None,
    cli_path: str | Path | None = None,
):
    self._api_key = api_key or os.getenv("ANTHROPIC_API_KEY")
    self._permission_mode = permission_mode
    self._model = model
    self._cwd = str(Path(cwd).expanduser()) if cwd else os.getcwd()
    self._cli_path = str(Path(cli_path).expanduser()) if cli_path else None
    self._rate_limit_bucket = self._build_rate_limit_bucket()
```

### Default Tools

```python
DEFAULT_TOOLS = ["Read", "Write", "Edit", "Bash", "Glob", "Grep"]
```

Tool detail extractor (사용자 가시 메시지용):

```python
_TOOL_DETAIL_EXTRACTORS = {
    "Read": "file_path",
    "Glob": "pattern",
    "Grep": "pattern",
    "Edit": "file_path",
    "Write": "file_path",
    "Bash": "command",
    "WebFetch": "url",
    "WebSearch": "query",
    "NotebookEdit": "notebook_path",
}
```

→ "Read: src/foo.py" 같은 휴먼 친화 표시.

### Transient Error Detection

```python
TRANSIENT_ERROR_PATTERNS = (
    "concurrency", "rate limit", "429", "500", "502", "503", "504",
    "timeout", "connection", "exit code 1",
)
```

매칭 시 exponential backoff (1.0 → 10.0s, MAX_RETRIES=3) + heartbeat AgentMessage emit.

### Shared Rate Limit Bucket

```python
class SharedRateLimitBucket:
    def __init__(self, runtime_backend, request_limit, token_limit): ...
    
    async def acquire(self, estimated_tokens: int) -> tuple[float, RateLimitSnapshot]: ...
    async def force_reserve(self, estimated_tokens: int) -> RateLimitSnapshot: ...
```

타임아웃 초과 후 `force_reserve` — 동시 timeout-fallback 들이 모두 bypass 하면 N× burst → upstream 폭발 방지.

Heartbeat 5s 마다 system AgentMessage emit (subtype `rate_limit_backoff` 또는 `rate_limit_timeout_force_reserve`).

### MCP Delegation Hook

`_build_delegated_tool_context_update(hook_input, effective_tools)` — `ouroboros_execute_seed` MCP 콜에 부모 Claude 세션 메타 자동 주입:

```python
DELEGATED_PARENT_SESSION_ID_ARG = "_ooo_parent_claude_session_id"
DELEGATED_PARENT_TRANSCRIPT_PATH_ARG = "_ooo_parent_claude_transcript_path"
DELEGATED_PARENT_CWD_ARG = "_ooo_parent_claude_cwd"
DELEGATED_PARENT_PERMISSION_MODE_ARG = "_ooo_parent_claude_permission_mode"
DELEGATED_PARENT_EFFECTIVE_TOOLS_ARG = "_ooo_parent_effective_tools"
```

Matcher:
```
mcp__plugin_ouroboros_ouroboros__ouroboros_execute_seed |
mcp__plugin_ouroboros_ouroboros__ouroboros_start_execute_seed |
mcp__ouroboros__ouroboros_execute_seed |
mcp__ouroboros__ouroboros_start_execute_seed |
ouroboros_execute_seed |
ouroboros_start_execute_seed
```

→ delegated execute-seed 가 부모 세션 컨텍스트 inherit.

### `complete()` LLM Adapter Bridge

LLMAdapter Protocol 호환 — InterviewEngine 같은 컴포넌트가 `complete()` 기대 시.

```python
async def complete(self, messages: list[Message], config: CompletionConfig) -> Result[CompletionResponse, ProviderError]: ...
```

System prompt 추출 → execute_task 호출 (read-only tools: Read/Glob/Grep) → assistant text 수집 → CompletionResponse return.

빈 응답 → ProviderError ("Empty response from Claude Agent SDK").

## Runtime Factory (`runtime_factory.py`)

```python
def create_agent_runtime(
    *,
    backend: str | None = None,
    permission_mode: str | None = None,
    model: str | None = None,
    cli_path: str | Path | None = None,
    cwd: str | Path | None = None,
    llm_backend: str | None = None,
) -> AgentRuntime:
    resolved_backend = resolve_agent_runtime_backend(backend)
    resolved_permission_mode = permission_mode or get_agent_permission_mode(backend=resolved_backend)
    resolved_llm_backend = llm_backend or get_llm_backend()
    
    if resolved_backend == "claude":
        return ClaudeAgentAdapter(permission_mode, model, cwd, cli_path or get_cli_path())
    
    runtime_kwargs = {
        "permission_mode": resolved_permission_mode,
        "model": model,
        "cwd": cwd,
        "skill_dispatcher": create_codex_command_dispatcher(cwd, runtime_backend=resolved_backend, llm_backend=resolved_llm_backend),
        "llm_backend": resolved_llm_backend,
    }
    
    if resolved_backend == "codex":
        return CodexCliRuntime(cli_path=cli_path or get_codex_cli_path(), **runtime_kwargs)
    if resolved_backend == "opencode":
        return OpenCodeRuntime(cli_path=cli_path or get_opencode_cli_path(), opencode_mode="subprocess", **runtime_kwargs)
    if resolved_backend == "hermes":
        return HermesCliRuntime(cli_path=cli_path or get_hermes_cli_path(), **runtime_kwargs)
```

## OpenCode 의 `opencode_mode="subprocess"` 하드코딩 이유

```
OpenCodeRuntime is the SUBPROCESS orchestrator (`ouroboros run`).
It shells out to `opencode run --pure` — no bridge plugin exists in that context.
Hardcode "subprocess" so handlers never emit dead _subagent envelopes,
regardless of what config.yaml says.

Plugin mode is exclusively an MCP-server concern (composition root in
create_ouroboros_server reads config there).
```

→ subprocess vs plugin 컨텍스트 명확히 분리. subprocess 에선 _subagent 페이로드 의미 없음 (해당 envelope 처리할 bridge 없음).

## Provider 어댑터 분리

`src/ouroboros/providers/` — runtime 어댑터와 별개. LLM completion 호출 추상.

| Provider | 모듈 | 역할 |
|---|---|---|
| LLMAdapter Protocol | `base.py` | `Message`, `MessageRole`, `CompletionConfig`, `CompletionResponse`, `UsageInfo` |
| LiteLLM | `litellm_adapter.py` | 100+ provider |
| Anthropic SDK | `anthropic_adapter.py` | 직접 |
| Claude Code | `claude_code_adapter.py` | Claude Code 변형 |
| Codex CLI | `codex_cli_adapter.py` + `codex_cli_stream.py` | NDJSON 스트림 |
| OpenCode | `opencode_adapter.py` | |
| Gemini CLI | `gemini_cli_adapter.py` | |
| Factory | `factory.py` | `create_llm_adapter()` |

Runtime 어댑터 (orchestrator/) = task 실행. Provider 어댑터 (providers/) = LLM completion 호출. 두 추상 분리.

## 검증

`tests/unit/orchestrator/test_*` — 24 파일 (adapter, capabilities, codex_cli_runtime, codex_recursion_guard, command_dispatcher, control_plane, coordinator, dependency_analyzer, events, execution_runtime_scope, execution_strategy, hermes_runtime, inflight_cancellation, level_context, mcp_config, mcp_tools, opencode_runtime, parallel_executor 4 변형, policy, rate_limit, runner, runner_cancellation, runtime_factory, runtime_message_projection, sandbox_class, session, workflow_state).

E2E (`test_full_workflow.py` 496 LOC) 가 4 백엔드 모두 검증.

## Configuration

```yaml
orchestrator:
  runtime_backend: claude   # claude | codex | opencode | hermes
  permission_mode: acceptEdits   # acceptEdits | bypassPermissions | default
  cli_path: null              # auto-detect
```

Env override:
- `OUROBOROS_AGENT_RUNTIME` — 백엔드
- `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`
- `OUROBOROS_ANTHROPIC_RPM_CEILING` / `OUROBOROS_ANTHROPIC_TPM_CEILING`

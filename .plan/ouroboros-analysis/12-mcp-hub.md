# 12. MCP Hub (Bidirectional)

## 책임

Ouroboros 는 **양방향 MCP 허브** — 서버 (도구 노출) + 클라이언트 (외부 도구 소비) + server-to-server bridge.

## 위치 (`src/ouroboros/mcp/`)

```
mcp/
├─ errors.py                                    # MCPError 트리
├─ types.py                                      # TransportType, ContentType, MCPServerConfig 등
├─ job_manager.py                                # async job lifecycle
├─ server/ (4)
│   ├─ protocol.py                                # MCPServer Protocol + ToolHandler/ResourceHandler/PromptHandler
│   ├─ adapter.py                                 # FastMCP 어댑터
│   ├─ security.py                                # InputValidator, FREETEXT_FIELDS
│   └─ __init__.py
├─ client/ (4)
│   ├─ manager.py                                 # MCPClientManager 외부 서버 풀
│   ├─ adapter.py                                 # transport lifecycle
│   ├─ protocol.py
│   └─ __init__.py
├─ bridge/ (3)
│   ├─ bridge.py                                  # server-to-server bridging
│   ├─ config.py
│   └─ factory.py
├─ tools/ (16)
│   ├─ definitions.py                             # 도구 카탈로그
│   ├─ registry.py                                # ToolRegistry (전역)
│   ├─ ac_tree_hud_handler.py                      # AC 트리 HUD 핸들러
│   ├─ ac_tree_hud_render.py                       # status icons, depth2/3 렌더, truncation
│   ├─ authoring_handlers.py                       # seed 작성 도구
│   ├─ brownfield_handler.py                       # scan, set_defaults, list, detect
│   ├─ bridge_mixin.py                             # 다른 MCP 서버 브리지 mixin
│   ├─ dashboard.py                                # TUI HUD 피드
│   ├─ evaluation_handlers.py                       # ouroboros_evaluate, measure_drift
│   ├─ evolution_handlers.py                       # evolve_step, lateral_think
│   ├─ execution_handlers.py                       # execute_seed, start_execute_seed
│   ├─ job_handlers.py                             # job_wait, job_result, cancel
│   ├─ pm_handler.py                               # PM 트랙
│   ├─ qa.py                                       # QA judge
│   ├─ query_handlers.py                           # query_events, session_status
│   └─ subagent.py                                 # _subagent / _subagents envelope
└─ resources/
    └─ handlers.py                                # MCP resource (gitnexus://...) 스타일
```

## 서버 모드

```bash
ouroboros mcp serve              # stdio
ouroboros mcp serve --transport sse --host 0.0.0.0 --port 8080
ouroboros mcp serve --transport streamable-http --host 127.0.0.1 --port 9000
```

진입: `cli/commands/mcp.py` → `mcp/server/adapter.py` (FastMCP 기반).

`.claude-plugin/.mcp.json` 등록:
```json
{"mcpServers": {"ouroboros": {"command": "uvx", "args": ["--from", "ouroboros-ai[mcp,claude]", "ouroboros", "mcp", "serve"]}}}
```

## 클라이언트 모드

```bash
ouroboros run --mcp-config mcp.yaml seed.yaml
```

외부 MCP 서버 도구 가져와서 실행 컨텍스트에 머지.

### Tool 우선순위

1. **Built-in 항상 wins** (Ouroboros 내장 도구)
2. 첫 MCP 서버 wins (중복 시)
3. `--mcp-tool-prefix` 로 namespace

### 예시 외부 도구

- filesystem MCP
- github MCP
- 데이터베이스 MCP
- 사용자 정의

## Protocol Definitions (`mcp/server/protocol.py`)

```python
class ToolHandler(Protocol):
    name: str
    description: str
    async def __call__(self, **kwargs) -> MCPToolResult: ...

class ResourceHandler(Protocol):
    uri_template: str
    async def fetch(self, uri: str) -> MCPResourceContent: ...

class PromptHandler(Protocol):
    name: str
    async def render(self, **kwargs) -> str: ...

class MCPServer(Protocol):
    def register_tool(self, handler: ToolHandler) -> None: ...
    def register_resource(self, handler: ResourceHandler) -> None: ...
    def register_prompt(self, handler: PromptHandler) -> None: ...
    async def run(self, *, transport, host, port, credentials=None) -> None: ...
```

## ToolRegistry (`mcp/tools/registry.py`)

```python
class ToolRegistry:
    def register(self, definition: MCPToolDefinition, handler: ToolHandler) -> None: ...
    def get(self, name: str) -> ToolHandler | None: ...
    def list(self) -> list[MCPToolDefinition]: ...

def get_global_registry() -> ToolRegistry: ...

def register_tool(definition, handler) -> None:
    """Decorator-friendly 전역 등록"""
```

## MCP Types (`mcp/types.py`)

```python
class TransportType(StrEnum):
    STDIO = "stdio"
    SSE = "sse"
    STREAMABLE_HTTP = "streamable-http"

class ContentType(StrEnum):
    TEXT = "text"
    IMAGE = "image"
    RESOURCE = "resource"

class MCPServerConfig(BaseModel, frozen=True):
    name: str
    transport: TransportType
    command: str | None = None       # stdio
    args: list[str] = []
    url: str | None = None            # sse / streamable-http
    env: dict[str, str] = {}
    timeout: int = 30
    headers: dict[str, str] = {}

class MCPToolDefinition(BaseModel, frozen=True):
    name: str
    description: str
    parameters: dict[str, Any]        # JSON Schema
    server_name: str | None = None    # 외부 서버 출처

class MCPToolResult(BaseModel):
    content: list[MCPContent]
    is_error: bool = False
    meta: dict[str, Any] = {}

class MCPCapabilities(BaseModel, frozen=True):
    tools: bool = True
    resources: bool = False
    prompts: bool = False
    logging: bool = False
```

## Error 트리 (`mcp/errors.py`)

```
MCPError ⊂ OuroborosError
├─ MCPClientError
│   ├─ MCPConnectionError(transport)
│   ├─ MCPTimeoutError(timeout_seconds, operation)
│   └─ MCPProtocolError
└─ MCPServerError
    ├─ MCPAuthError
    ├─ MCPResourceNotFoundError
    └─ MCPToolError(tool_name, error_code)
```

## Job Manager (`mcp/job_manager.py`)

비동기 job lifecycle 관리.

```python
class JobManager:
    async def start_job(self, kind, args) -> JobHandle: ...
    async def wait(self, job_id, cursor, timeout_seconds) -> JobUpdate: ...
    async def result(self, job_id) -> JobResult: ...
    async def cancel(self, job_id) -> bool: ...

class JobHandle:
    job_id: str
    cursor: int
    
class JobUpdate:
    cursor: int
    status: Literal["running", "completed", "failed", "cancelled"]
    changed: bool
    text: str          # 사람-가시 progress
    meta: dict          # current_phase, ac_completed, ac_total, sub_ac_completed, sub_ac_total

class JobResult:
    text: str
    meta: dict
    is_error: bool
```

`run/SKILL.md` 가 자세한 사용 패턴 — 200s+ 블로킹 회피 위해 `view: "summary"` + `timeout_seconds: 180` long-poll.

## 노출 도구 — 23 핸들러 (Section 28 정정)

`mcp/tools/definitions.py` 의 `get_ouroboros_tools()` 가 반환하는 `OuroborosToolHandlers` tuple 정확한 카탈로그:

| # | 핸들러 클래스 | 모듈 | 설명 |
|---|---|---|---|
| 1 | `ExecuteSeedHandler` | execution_handlers.py | 시드 실행 (sync) |
| 2 | `StartExecuteSeedHandler` | execution_handlers.py | 시드 실행 (async, returns job_id) |
| 3 | `SessionStatusHandler` | query_handlers.py | 세션 상태 조회 |
| 4 | `JobStatusHandler` | job_handlers.py | job 상태 |
| 5 | `JobWaitHandler` | job_handlers.py | job long-poll (180 sec default) |
| 6 | `JobResultHandler` | job_handlers.py | job 결과 fetch |
| 7 | `ACTreeHUDHandler` | ac_tree_hud_handler.py | AC 트리 HUD (max_nodes=30) |
| 8 | `CancelJobHandler` | job_handlers.py | job 취소 |
| 9 | `QueryEventsHandler` | query_handlers.py | event_store 쿼리 |
| 10 | `GenerateSeedHandler` | authoring_handlers.py | 인터뷰 → Seed 변환 |
| 11 | `MeasureDriftHandler` | evaluation_handlers.py | drift 측정 |
| 12 | `InterviewHandler` | authoring_handlers.py | dev interview 진행 |
| 13 | `EvaluateHandler` | evaluation_handlers.py | mechanical+semantic+consensus 3-stage 평가 |
| 14 | `ChecklistVerifyHandler` | evaluation_handlers.py | AC 검증 (`evaluate_handler` 의존) |
| 15 | `LateralThinkHandler` | evaluation_handlers.py | 5 페르소나 lateral thinking + fan-out |
| 16 | `EvolveStepHandler` | evolution_handlers.py | 진화 1 세대 (sync) |
| 17 | `StartEvolveStepHandler` | evolution_handlers.py | 진화 1 세대 (async) |
| 18 | `LineageStatusHandler` | evolution_handlers.py | 진화 lineage 상태 |
| 19 | `EvolveRewindHandler` | evolution_handlers.py | 진화 rewind to 세대 N |
| 20 | `CancelExecutionHandler` | job_handlers.py | 실행 취소 (in-flight, cancellation_registry) |
| 21 | `BrownfieldHandler` | brownfield_handler.py | scan / set_defaults / list / detect |
| 22 | `PMInterviewHandler` | pm_handler.py | PM 트랙 인터뷰 |
| 23 | `QAHandler` | qa.py | QA judge (pass/revise/fail) |

별도 모듈에서 등록될 수 있는 추가 도구 (예: `subagent.py` 의 `_subagent` / `_subagents` envelope, `dashboard.py` 의 dashboard feed) 는 `OuroborosToolHandlers` tuple 에 포함 안 됨 — 이들은 `bridge_mixin.py` 또는 동적 dispatcher 를 통해 별도 등록.

### 도구 이름 매핑 (handler.name → MCP tool name)

각 핸들러의 `name` 속성이 실 MCP 도구 이름. `definitions.py` 가 직접 `"name"` 문자열을 가진 dict 형태가 아니라 핸들러 인스턴스를 tuple 로 반환 — `MCPToolDefinition` 빌드는 `mcp_tools.py` 의 `enumerate_runtime_builtin_tool_definitions()` + `assemble_session_tool_catalog()` 에서 수행. 이 단계에서 `mcp_tool_prefix` 가 적용되면 모든 도구 이름에 prefix 추가.

핸들러 → 추정 노출 이름:
- `ExecuteSeedHandler` → `ouroboros_execute_seed`
- `StartExecuteSeedHandler` → `ouroboros_start_execute_seed`
- `SessionStatusHandler` → `ouroboros_session_status`
- `JobStatusHandler` → `ouroboros_job_status`
- `JobWaitHandler` → `ouroboros_job_wait`
- `JobResultHandler` → `ouroboros_job_result`
- `ACTreeHUDHandler` → `ouroboros_ac_tree_hud`
- `CancelJobHandler` → `ouroboros_cancel_job`
- `QueryEventsHandler` → `ouroboros_query_events`
- `GenerateSeedHandler` → `ouroboros_generate_seed`
- `MeasureDriftHandler` → `ouroboros_measure_drift`
- `InterviewHandler` → `ouroboros_interview`
- `EvaluateHandler` → `ouroboros_evaluate`
- `ChecklistVerifyHandler` → `ouroboros_checklist_verify`
- `LateralThinkHandler` → `ouroboros_lateral_think`
- `EvolveStepHandler` → `ouroboros_evolve_step`
- `StartEvolveStepHandler` → `ouroboros_start_evolve_step`
- `LineageStatusHandler` → `ouroboros_lineage_status`
- `EvolveRewindHandler` → `ouroboros_evolve_rewind`
- `CancelExecutionHandler` → `ouroboros_cancel_execution`
- `BrownfieldHandler` → `ouroboros_brownfield`
- `PMInterviewHandler` → `ouroboros_pm_interview`
- `QAHandler` → `ouroboros_qa`

(handler 내부 `name` 필드 직접 확인 못 함 — Section 27 의 "도구 이름 grep `'"name":'` 빈 결과" 참조. `definitions.py` 가 tuple-of-instances 형식이라 grep 안 잡힘.)

### `opencode_mode` 분기

`get_ouroboros_tools(*, runtime_backend, llm_backend, mcp_manager, mcp_tool_prefix, opencode_mode)`:

```python
"""
opencode_mode 가 "plugin" 이고 runtime_backend 가 OpenCode 변종이면
모든 _subagent 디스패치 핸들러는 envelope 만 반환 (실 호출 안 함).
그 외 (None 포함) 모든 조합은 in-process 실 호출.
참조: ouroboros.mcp.tools.subagent.should_dispatch_via_plugin
"""
```

→ OpenCode plugin mode 일 때 `_subagent` 도구가 envelope-only 가 되고, 실 dispatch 는 OpenCode TS bridge 가 fire-and-forget 으로 처리 (Section 16).

## AC Tree HUD 도구 상세

`ouroboros_ac_tree_hud(session_id, cursor, view, max_nodes=30)`:

| view | 출력 |
|---|---|
| `compact` | 한 줄 — `job_x | running | AC 3/17` |
| `summary` | 메시지 + AC/Sub-AC 카운트 |
| `tree` | 전체 트리 (max_nodes 절단) |

`ac_tree_hud_render.py` 의 status icons:
- ✓ done
- ✗ failed
- ▶ running
- ⏸ paused
- ⊘ cancelled
- ? unknown

테스트 11종 (`tests/unit/mcp/tools/test_ac_tree_hud_*`):
- footer
- handler 기본 / completed / cursor_changed / invalid_session / no_execution / waiting
- max_nodes
- render depth2 / depth3
- status_icons
- truncation

## Lateral Think v0.30.0 — Multi-persona Fan-out

CHANGELOG `[Unreleased]`:
```
ouroboros_lateral_think 가 persona="all" 또는 personas=["hacker","architect",...] 수용
```

내부 흐름:
1. `build_lateral_multi_subagent()` — 페르소나별 subagent envelope 생성
2. `_subagents` (plural) JSON 배열 페이로드
3. opencode bridge `MAX_FANOUT=10` 병렬 `promptAsync`
4. 페이로드별 dedupe + truncation + validation
5. 한 페르소나 실패 → 나머지 계속
6. 응답에 `ouroboros_subagents`, `ouroboros_dispatch_errors` 메타

각 페르소나가 독립 LLM 컨텍스트 → anchoring bias 제거.

## Security (`mcp/server/security.py`)

`InputValidator` — DoS + injection 방지.

`FREETEXT_FIELDS` 화이트리스트 (사용자 자연어 prose):
- goals
- prompts
- descriptions

→ shell metacharacters (`;`, `|`, `&`, backticks, `$()`) 정상 prose. structural 필드만 strict.

`AuthContext` — `MappingProxyType` (`metadata` frozen dict).

(`[0.13.3]` fix: 중첩 string 값까지 재귀 검증)

## Bridge (`mcp/bridge/`)

server-to-server — Ouroboros 가 다른 MCP 서버를 own 도구처럼 노출.

```python
class BridgeConfig:
    upstream: MCPServerConfig
    tool_prefix: str | None = None
    expose_resources: bool = False

class Bridge:
    async def proxy_call(self, tool_name, args) -> MCPToolResult: ...
```

테스트: `tests/integration/mcp/bridge/test_bridge_server_to_server.py`, `tests/unit/mcp/bridge/*` (5).

## Skill 활용

대부분 SKILL.md 가 ToolSearch 로 deferred 도구 강제 로드:

```
ToolSearch query: "+ouroboros interview"
ToolSearch query: "+ouroboros execute"
ToolSearch query: "+ouroboros brownfield"
```

→ 도구 schema 로드 후 호출 가능.

## CLI 진입점 (`cli/commands/mcp.py` + `mcp_doctor.py`)

```bash
ouroboros mcp serve           # 서버 시작
ouroboros mcp doctor          # 헬스체크 진단
```

`mcp_doctor.py` — 클라이언트 → 서버 round-trip 검사, transport 통신 검증, capability 보고.

## 검증

테스트 다수 (`tests/unit/mcp/`):
- `bridge/` 5
- `client/` 3
- `server/` 2
- `tools/` 15
- `test_errors.py`, `test_job_manager.py`, `test_types.py`

`tests/integration/mcp/` 5 — bridge_server_to_server, client_adapter, client_manager, server_adapter.

## CLAUDE.md 슬로건

> **Ouroboros functions as a bidirectional MCP Hub**
> - Server mode: Exposes ouroboros_execute_seed, ouroboros_session_status, ouroboros_query_events to Claude Desktop and other MCP clients
> - Client mode: Discovers and consumes tools from external MCP servers (filesystem, GitHub, databases), merged with built-in tools

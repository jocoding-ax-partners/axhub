# 34. TUI Views + Permissions + MCP Bridge body

> Round 2 review (gap-finder) 발견 — Section 15 (TUI) 가 디렉토리 트리만 cover, views/ 본문 0 분석. Permissions + MCP bridge 본문도 미커버. 이 section 이 보완.

## 34.1 Rust TUI views (`crates/ouroboros-tui/src/views/`)

5 view + 1 mod.rs:

| 파일 | 크기 | 주요 함수 |
|---|---|---|
| `dashboard.rs` | 14.6 KB | `pub fn render(ui, state)` — 6 phase card + AC progress + drift + cost summary |
| `execution.rs` | 12.5 KB | `pub fn render(ui, state)` — 3 horizontal card (Execution / Drift / Cost) + AC tree |
| `lineage.rs` | 22.4 KB | `render` + 11 helper (`render_lineage_detail`, `render_gen_detail`, `render_double_diamond`, `gen_phase_color`, `score_color`, `status_color`, `rebuild_gen_list_by_idx`) |
| `logs.rs` | 2.7 KB | `pub fn render_log_panel(ui, state)` — log tail viewer |
| `session_selector.rs` | 3.8 KB | `pub fn render(ui, state) -> Option<usize>` — modal selection, 선택 시 idx 반환 |
| `mod.rs` | 95 B | re-export only |

### dashboard.rs body 분석

핵심 layout:

```
ui.container().grow(1).gap(1).col:
  Phase Outputs (horizontal cards)
    for phase in Phase::ALL:
      icon: ● (done) / ◐ (active) / ○ (pending)
      color: success / accent / dim
      max 3 outputs per phase + "+N more"
  AC progress + metrics
    [done/total AC] + ratio progress bar
    elapsed time
```

`rebuild_tree_state()` 가 `populate_state_from_events()` 에서만 호출 — 매 frame 호출하면 expand/collapse 상태 파괴.

theme: `Color::Rgb(235, 111, 146)` = Rose Pine love (error red).

### execution.rs body 분석

3 horizontal card layout:
- **Execution card**: Exec ID / Session ID / Status (icon + label + color) / Phase / Iteration
- **Drift card**: Combined / Goal / Constraint / Ontology + sparkline history (20 sample)
- **Cost card**: (생략된 부분, sparkline 으로 cost history)

`drift_kv()` helper — 일관된 key-value 형식.

### lineage.rs body 분석 (가장 큰 view, 22.4 KB)

11 helper function:
- `rebuild_gen_list_by_idx(state, lin_idx)` — generation list 재구축
- `render_lineage_detail` — selected lineage 의 detail panel
- `render_gen_detail` — selected generation 의 detail
- `render_double_diamond` — Phase 2 의 double diamond 시각화 (4 phase: Discover / Define / Design / Deliver)
- `gen_phase_to_index(phase)` — phase string → index
- `is_gen_phase_done(phase)` — 완료 여부
- `gen_phase_color(slot, current, done, dim, accent, success)` — phase 별 색
- `score_color(score, success, warning, error)` — score 0~1 → color (전형적 traffic light)
- `status_color(status, ...)` — AC status → color
- `placeholder(ui, msg, ...)` — empty state

### logs.rs (가장 작은, 2.7 KB)

단일 `render_log_panel` — log entry list with severity color coding.

### session_selector.rs

modal — selection 시 `Option<usize>` 반환 (None = no selection).

## 34.2 Python TUI widgets/screens (`src/ouroboros/tui/`)

### widgets/ (8 widget — `__init__.py` 제외)

| 위젯 | 크기 | 역할 |
|---|---|---|
| `ac_progress.py` | 7.6 KB | AC 별 progress bar |
| `ac_tree.py` | 15.4 KB | hierarchical AC tree (가장 큰) |
| `agent_activity.py` | 5.0 KB | agent 별 활동 indicator |
| `cost_tracker.py` | 8.0 KB | per-tier token + $ 추적 |
| `drift_meter.py` | 10.8 KB | drift 3-component meter |
| `lineage_tree.py` | 8.8 KB | lineage 시각화 |
| `parallel_graph.py` | 10.5 KB | parallel execution graph |
| `phase_progress.py` | 8.6 KB | phase 별 progress |

### screens/ (10 screen — `__init__.py` 제외)

| 스크린 | 크기 | 역할 |
|---|---|---|
| `dashboard.py` | 20.5 KB | 메인 dashboard v1 |
| `dashboard_v2.py` | 24.1 KB | v2 — improved layout |
| `dashboard_v3.py` | 32.6 KB | v3 — current (가장 큰) |
| `execution.py` | 13.9 KB | execution view |
| `hud_dashboard.py` | 17.6 KB | HUD overlay |
| `lineage_detail.py` | 20.3 KB | lineage detail screen |
| `lineage_selector.py` | 5.0 KB | lineage selection modal |
| `logs.py` | 12.7 KB | log viewer |
| `session_selector.py` | 6.3 KB | session selector |
| `confirm_rewind.py` | 3.8 KB | rewind confirmation modal |
| `debug.py` | 10.3 KB | debug tools |

> **Dashboard 진화**: v1 → v2 → v3 = 점진 개선. v3 가 활성. v1/v2 는 legacy 유지 (제거 후보 — Section 24 의 "dead code" 후보 추적).

## 34.3 Permissions (sandbox 변환 layer)

> Section 11 (runtime abstraction) 이 SandboxClass enum 만 언급, 실제 backend 별 변환 layer 본문 미분석. 이 절이 보완.

### `src/ouroboros/claude_permissions.py` (59 LOC)

**역할**: SandboxClass → Claude SDK `permission_mode` 변환.

**type alias**:
```python
ClaudePermissionMode = Literal["default", "acceptEdits", "bypassPermissions"]
```

**매핑** (identity for legacy strings):
```python
_SANDBOX_TO_CLAUDE_MODE: dict[SandboxClass, ClaudePermissionMode] = {
    SandboxClass.READ_ONLY: "default",
    SandboxClass.WORKSPACE_WRITE: "acceptEdits",
    SandboxClass.UNRESTRICTED: "bypassPermissions",
}
```

**핵심 함수**:
```python
def claude_permission_mode_for_sandbox(sandbox: SandboxClass) -> ClaudePermissionMode:
    mode = _SANDBOX_TO_CLAUDE_MODE.get(sandbox)
    if mode is None:
        # Fail-loud: enum 추가 시 invariant test 가 깨지도록
        raise KeyError(f"No Claude SDK permission_mode registered for sandbox class {sandbox!r}")
    if sandbox is SandboxClass.UNRESTRICTED:
        log.warning("permissions.bypass_activated", sandbox=sandbox.value)
    return mode
```

**Design 원칙**:
- "엔진이 SandboxClass 의 owner". Claude 의 mode string 은 "Claude-specific 변환".
- legacy 가 Claude vocabulary 채택 했었기에 identity mapping. 새 mapping 은 변환.
- `UNRESTRICTED` 사용 시 항상 `permissions.bypass_activated` warning log → audit trail.
- KeyError = fail-loud. silent default 안 함 (security 위험).

### `src/ouroboros/codex_permissions.py` (107 LOC)

**역할**: SandboxClass → Codex CLI flag 변환.

**매핑**:
```python
_SANDBOX_TO_CODEX_ARGS: dict[SandboxClass, list[str]] = {
    SandboxClass.READ_ONLY: ["--sandbox", "read-only"],
    SandboxClass.WORKSPACE_WRITE: ["--full-auto"],
    SandboxClass.UNRESTRICTED: ["--dangerously-bypass-approvals-and-sandbox"],
}
```

→ Codex flag `--dangerously-bypass-approvals-and-sandbox` 의 명시적 위험 명칭. UX 가 명백히 "위험" 신호.

**역방향 매핑**:
```python
_PERMISSION_MODE_TO_SANDBOX: dict[CodexPermissionMode, SandboxClass] = {
    "default": SandboxClass.READ_ONLY,
    "acceptEdits": SandboxClass.WORKSPACE_WRITE,
    "bypassPermissions": SandboxClass.UNRESTRICTED,
}
```

**핵심 함수**:
- `resolve_codex_permission_mode(permission_mode, default_mode="default")` — string → validated mode (3 의 frozenset 멤버십 체크)
- `build_codex_exec_args_for_sandbox(sandbox)` — SandboxClass → flags
- `build_codex_exec_permission_args(permission_mode, default_mode)` — string → SandboxClass → flags (legacy 호환 wrapper)

**Invariant**: 모든 SandboxClass enum 멤버가 mapping 에 있어야 함. 누락 시 `KeyError` (test 가 검증).

### Cross-runtime 일관성 보장

- 두 file 모두 `from ouroboros.sandbox import SandboxClass` — **single source of truth**
- 두 file 모두 `UNRESTRICTED` 사용 시 `log.warning("permissions.bypass_activated", ...)` — 동일 event name → audit dashboard 통합 가능
- 두 file 모두 missing entry 시 `KeyError` — fail-loud invariant
- Hermes / OpenCode 도 같은 pattern 으로 추가 가능 (현재 없음 — 두 backend 가 sandbox enum 사용 안 함, gap)

## 34.4 MCP Bridge body (`mcp/bridge/bridge.py` + `mcp/tools/bridge_mixin.py`)

> Section 12 (MCP hub) 가 bridge 의 interface 만 언급, 실제 lifecycle 코드 본문 미분석.

### `mcp/bridge/bridge.py` (88 LOC) — `MCPBridge` 클래스

**역할**: server-to-server MCP 연결의 lifecycle 관리.

**dataclass + lazy init**:
```python
@dataclass
class MCPBridge:
    config: MCPBridgeConfig
    _manager: MCPClientManager = field(init=False, repr=False)
    _connected: bool = field(default=False, init=False, repr=False)

    def __post_init__(self) -> None:
        self._manager = MCPClientManager(
            max_retries=self.config.retry_attempts,
            health_check_interval=self.config.health_check_interval,
            default_timeout=self.config.timeout_seconds,
        )
```

**factory methods**:
- `MCPBridge.from_config(config)` — already-loaded config
- `MCPBridge.from_config_file(path)` — load + raise if invalid

**properties**:
- `manager` → `MCPClientManager` 노출
- `is_connected` → bool
- `tool_prefix` → config 의 prefix (외부 tool 의 namespace 충돌 방지)

**lifecycle**:
```python
async def connect(self) -> dict[str, Result[MCPServerInfo, Any]]:
    if self._connected:
        log.warning("bridge.already_connected")
        return {}
    for server_config in self.config.servers:
        await self._manager.add_server(server_config)
    results = await self._manager.connect_all()
    connected_count = sum(1 for r in results.values() if r.is_ok)
    log.info("bridge.connected", connected=connected_count, total=len(results), servers=...)
    self._connected = True
    return results

async def disconnect(self) -> None:
    if not self._connected:
        return
    await self._manager.disconnect_all()
    self._connected = False
    log.info("bridge.disconnected")

async def close(self) -> None:
    await self.disconnect()
```

**async context manager**:
```python
async def __aenter__(self) -> MCPBridge:
    await self.connect()
    return self

async def __aexit__(self, *exc: object) -> None:
    await self.disconnect()
```

→ `async with MCPBridge.from_config_file(path) as bridge: ...` 패턴 지원.

**Result envelope**: `connect()` 반환 = `dict[server_name, Result[MCPServerInfo, Any]]` — partial failure 가능 (몇 server 성공, 몇 실패). 호출자가 `is_ok` 체크.

### `mcp/tools/bridge_mixin.py` (52 LOC) — `BridgeAwareMixin`

**역할**: tool handler 가 외부 MCP server 접근 필요할 때 dependency inject 받는 mixin.

**dataclass mixin**:
```python
@dataclass
class BridgeAwareMixin:
    mcp_manager: Any | None = field(default=None, repr=False)
    mcp_tool_prefix: str = ""
```

→ default `None` / `""` 이라 bridge 없이도 handler 가 동작 (graceful degradation).

**injection helper**:
```python
def inject_bridge(handler: object, bridge: object | None) -> bool:
    if bridge is None or not isinstance(handler, BridgeAwareMixin):
        return False
    handler.mcp_manager = getattr(bridge, "manager", None)
    handler.mcp_tool_prefix = getattr(bridge, "tool_prefix", "")
    return True
```

→ duck-typed `getattr` — bridge object 의 protocol 만 알면 됨. composition root 에서 loop-based injection.

**Usage 패턴**:
```python
@dataclass
class MyHandler(BridgeAwareMixin):
    other_field: str = ""

    async def handle(self, arguments):
        if self.mcp_manager:
            tools = await self.mcp_manager.list_all_tools()
```

→ `if self.mcp_manager:` null check pattern. bridge 없는 환경에서도 handler 코드 전체 동작.

## 34.5 Cross-reference 와 보강

### Section 11 보강 후보
- SandboxClass → 4 backend 변환 layer 의 일관성 (Claude / Codex 둘만 있음, Hermes / OpenCode gap)
- `UNRESTRICTED` 의 audit log event name 통일 (`permissions.bypass_activated`)

### Section 12 보강 후보
- bridge 의 lifecycle (connect/disconnect/async context manager)
- `BridgeAwareMixin` 의 dependency injection 패턴
- partial failure 의 `dict[str, Result[...]]` 반환 (connect_all 결과)

### Section 15 보강 후보
- Rust views 의 11 helper (lineage.rs)
- Python widgets 의 8 file (각 widget LOC + 책임)
- screens 의 10 file + dashboard v1/v2/v3 진화
- Rose Pine theme color 사용

## 34.6 발견된 design pattern

### Translation table 패턴
- `claude_permissions.py` + `codex_permissions.py` 둘 다 enum → backend-specific format 변환
- 같은 pattern 으로 Hermes / OpenCode 추가 가능 (현재 없음)
- single source of truth = `ouroboros.sandbox.SandboxClass`

### Fail-loud invariant
- enum 추가 시 mapping 누락 → `KeyError` raise
- silent default = security risk → 회피
- test 가 enum 의 모든 멤버에 대해 mapping 존재 검증

### Audit log event 통일
- `permissions.bypass_activated` event name 가 Claude / Codex 둘 다 동일
- 새 backend 추가 시 같은 event name 사용 → audit dashboard 통합 가능

### Lifecycle context manager
- `MCPBridge.__aenter__` / `__aexit__` — RAII 스타일
- explicit `connect()` / `disconnect()` 도 지원 (긴 lifecycle 위)
- `close()` = `disconnect()` alias (file-like API 호환)

### Dependency injection via mixin
- `BridgeAwareMixin` — 모든 handler 가 inherit 안 해도 됨 (opt-in)
- `inject_bridge()` 가 duck-typed — protocol 만 알면 됨
- composition root 가 loop 으로 모든 handler 에 inject 시도

## 34.7 미발견 영역 (이번 라운드도 못 본 것)

- `views/dashboard.rs` 의 cost/metrics 부분 후반부 (line 80+)
- `views/execution.rs` 의 cost card + AC tree 부분 (line 80+)
- `views/lineage.rs` 의 11 helper 의 body (시그니처만)
- 8 widget 의 body (LOC 만)
- 10 screen 의 body (LOC 만)
- `dashboard_v3.py` 32.6 KB 본문 (가장 큰 screen)
- `tui/app.py` (top-level Textual app)
- `tui/events.py` (event 시스템)
- `MCPClientManager` 본체 (`mcp/client/manager.py`)
- `MCPBridgeConfig` (`mcp/bridge/config.py`)

## 34.8 다음 ralph round 추천

1. **Section 35**: dashboard_v3.py 32.6 KB 본문 + 모든 widget body
2. **Section 36**: Rust views 의 11 helper body + theme system
3. **Section 37**: MCPClientManager + MCPBridgeConfig + retry/health-check 알고리즘
4. **Section 38**: SandboxClass enum body + 4 backend 의 일관성 audit (Hermes/OpenCode gap)
